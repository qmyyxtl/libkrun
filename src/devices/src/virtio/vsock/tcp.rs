use std::num::Wrapping;
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::{Arc, Mutex};

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::sys::socket::{
    accept, bind, connect, getpeername, listen, recv, send, setsockopt, shutdown, socket, sockopt,
    AddressFamily, InetAddr, IpAddr, Ipv4Addr, MsgFlags, Shutdown, SockAddr, SockFlag, SockType,
};
use nix::unistd::close;

use super::super::Queue as VirtQueue;
use super::defs;
use super::defs::uapi;
use super::muxer::{push_packet, MuxerRx};
use super::muxer_rxq::MuxerRxQ;
use super::packet::{
    TsiAcceptReq, TsiConnectReq, TsiGetnameRsp, TsiListenReq, TsiSendtoAddr, VsockPacket,
};
use super::proxy::{Proxy, ProxyError, ProxyStatus, ProxyUpdate, RecvPkt};
use utils::epoll::EventSet;

use vm_memory::GuestMemoryMmap;

pub struct TcpProxy {
    id: u64,
    cid: u64,
    parent_id: u64,
    local_port: u32,
    peer_port: u32,
    control_port: u32,
    fd: RawFd,
    pub status: ProxyStatus,
    mem: GuestMemoryMmap,
    queue_stream: Arc<Mutex<VirtQueue>>,
    queue_dgram: Arc<Mutex<VirtQueue>>,
    rxq_stream: Arc<Mutex<MuxerRxQ>>,
    rxq_dgram: Arc<Mutex<MuxerRxQ>>,
    rx_cnt: Wrapping<u32>,
    tx_cnt: Wrapping<u32>,
    last_tx_cnt_sent: Wrapping<u32>,
    peer_buf_alloc: u32,
    peer_fwd_cnt: Wrapping<u32>,
    push_cnt: Wrapping<u32>,
}

impl TcpProxy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        cid: u64,
        local_port: u32,
        peer_port: u32,
        control_port: u32,
        mem: GuestMemoryMmap,
        queue_stream: Arc<Mutex<VirtQueue>>,
        queue_dgram: Arc<Mutex<VirtQueue>>,
        rxq_stream: Arc<Mutex<MuxerRxQ>>,
        rxq_dgram: Arc<Mutex<MuxerRxQ>>,
    ) -> Result<Self, ProxyError> {
        let fd = socket(
            AddressFamily::Inet,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(ProxyError::CreatingSocket)?;

        // macOS forces us to do this here instead of just using SockFlag::SOCK_NONBLOCK above.
        match fcntl(fd, FcntlArg::F_GETFL) {
            Ok(flags) => match OFlag::from_bits(flags) {
                Some(flags) => {
                    if let Err(e) = fcntl(fd, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK)) {
                        warn!("error switching to non-blocking: id={}, err={}", id, e);
                    }
                }
                None => error!("invalid fd flags id={}", id),
            },
            Err(e) => error!("couldn't obtain fd flags id={}, err={}", id, e),
        };

        setsockopt(fd, sockopt::ReusePort, &true).map_err(ProxyError::SettingReusePort)?;

        Ok(TcpProxy {
            id,
            cid,
            parent_id: 0,
            local_port,
            peer_port,
            control_port,
            fd,
            status: ProxyStatus::Idle,
            mem,
            queue_stream,
            queue_dgram,
            rxq_stream,
            rxq_dgram,
            rx_cnt: Wrapping(0),
            tx_cnt: Wrapping(0),
            last_tx_cnt_sent: Wrapping(0),
            peer_buf_alloc: 0,
            peer_fwd_cnt: Wrapping(0),
            push_cnt: Wrapping(0),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_reverse(
        id: u64,
        cid: u64,
        parent_id: u64,
        local_port: u32,
        peer_port: u32,
        fd: RawFd,
        mem: GuestMemoryMmap,
        queue_stream: Arc<Mutex<VirtQueue>>,
        queue_dgram: Arc<Mutex<VirtQueue>>,
        rxq_stream: Arc<Mutex<MuxerRxQ>>,
        rxq_dgram: Arc<Mutex<MuxerRxQ>>,
    ) -> Self {
        debug!(
            "new_reverse: id={} local_port={} peer_port={}",
            id, local_port, peer_port
        );
        TcpProxy {
            id,
            cid,
            parent_id,
            local_port,
            peer_port,
            control_port: 0,
            fd,
            status: ProxyStatus::ReverseInit,
            mem,
            queue_stream,
            queue_dgram,
            rxq_stream,
            rxq_dgram,
            rx_cnt: Wrapping(0),
            tx_cnt: Wrapping(0),
            last_tx_cnt_sent: Wrapping(0),
            peer_buf_alloc: 0,
            peer_fwd_cnt: Wrapping(0),
            push_cnt: Wrapping(0),
        }
    }

    fn init_data_pkt(&self, pkt: &mut VsockPacket) {
        debug!(
            "tcp: init_data_pkt: id={}, local_port={}, peer_port={}",
            self.id, self.local_port, self.peer_port
        );
        pkt.set_op(uapi::VSOCK_OP_RW)
            .set_src_cid(uapi::VSOCK_HOST_CID)
            .set_dst_cid(self.cid)
            .set_src_port(self.local_port)
            .set_dst_port(self.peer_port)
            .set_type(uapi::VSOCK_TYPE_STREAM)
            .set_buf_alloc(defs::CONN_TX_BUF_SIZE as u32)
            .set_fwd_cnt(self.tx_cnt.0);
    }

    fn peer_avail_credit(&self) -> usize {
        (Wrapping(self.peer_buf_alloc as u32) - (self.rx_cnt - self.peer_fwd_cnt)).0 as usize
    }

    fn recv_to_pkt(&self, pkt: &mut VsockPacket) -> RecvPkt {
        if let Some(buf) = pkt.buf_mut() {
            let peer_credit = self.peer_avail_credit();
            let max_len = std::cmp::min(buf.len(), peer_credit);

            debug!(
                "recv_to_pkt: peer_avail_credit={}, buf.len={}, max_len={}",
                self.peer_avail_credit(),
                buf.len(),
                max_len,
            );

            if max_len == 0 {
                return RecvPkt::WaitForCredit;
            }

            match recv(self.fd, &mut buf[..max_len], MsgFlags::MSG_DONTWAIT) {
                Ok(cnt) => {
                    debug!("vsock: tcp: recv cnt={}", cnt);
                    if cnt > 0 {
                        debug!("vsock: tcp: recv rx_cnt={}", self.rx_cnt);
                        RecvPkt::Read(cnt)
                    } else {
                        RecvPkt::Close
                    }
                }
                Err(e) => {
                    debug!("vsock: tcp: recv_pkt: recv error: {:?}", e);
                    RecvPkt::Error
                }
            }
        } else {
            debug!("vsock: tcp: recv_pkt: pkt without buf");
            RecvPkt::Error
        }
    }

    fn recv_pkt(&mut self) -> (bool, bool) {
        let mut have_used = false;
        let mut wait_credit = false;
        let mut queue = self.queue_stream.lock().unwrap();

        while let Some(head) = queue.pop(&self.mem) {
            let len = match VsockPacket::from_rx_virtq_head(&head) {
                Ok(mut pkt) => match self.recv_to_pkt(&mut pkt) {
                    RecvPkt::WaitForCredit => {
                        wait_credit = true;
                        0
                    }
                    RecvPkt::Read(cnt) => {
                        self.rx_cnt += Wrapping(cnt as u32);
                        self.init_data_pkt(&mut pkt);
                        pkt.set_len(cnt as u32);
                        pkt.hdr().len() + cnt
                    }
                    RecvPkt::Close => {
                        self.status = ProxyStatus::Closed;
                        0
                    }
                    RecvPkt::Error => 0,
                },
                Err(e) => {
                    debug!("vsock: tcp: recv_pkt: RX queue error: {:?}", e);
                    0
                }
            };

            if len == 0 {
                queue.undo_pop();
                break;
            } else {
                have_used = true;
                self.push_cnt += Wrapping(len as u32);
                debug!(
                    "vsock: tcp: recv_pkt: pushing packet with {} bytes, push_cnt={}",
                    len, self.push_cnt
                );
                queue.add_used(&self.mem, head.index, len as u32);
            }
        }

        debug!("vsock: tcp: recv_pkt: have_used={}", have_used);
        (have_used, wait_credit)
    }

    fn push_connect_rsp(&self, result: i32) {
        debug!(
            "push_connect_rsp: id: {}, control_port: {}, result: {}",
            self.id, self.control_port, result
        );

        // This response goes to the control port (DGRAM).
        let rx = MuxerRx::ConnResponse {
            local_port: 1025,
            peer_port: self.control_port,
            result,
        };
        push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);
    }

    fn push_reset(&self) {
        debug!(
            "push_reset: id: {}, peer_port: {}, local_port: {}",
            self.id, self.peer_port, self.local_port
        );

        // This response goes to the connection.
        let rx = MuxerRx::Reset {
            local_port: self.local_port,
            peer_port: self.peer_port,
        };
        push_packet(
            self.cid,
            rx,
            &self.rxq_stream,
            &self.queue_stream,
            &self.mem,
        );
    }

    fn switch_to_connected(&mut self) {
        self.status = ProxyStatus::Connected;
        match fcntl(self.fd, FcntlArg::F_GETFL) {
            Ok(flags) => match OFlag::from_bits(flags) {
                Some(flags) => {
                    if let Err(e) = fcntl(self.fd, FcntlArg::F_SETFL(flags & !OFlag::O_NONBLOCK)) {
                        warn!("error switching to blocking: id={}, err={}", self.id, e);
                    }
                }
                None => error!("invalid fd flags id={}", self.id),
            },
            Err(e) => error!("couldn't obtain fd flags id={}, err={}", self.id, e),
        };
    }
}

impl Proxy for TcpProxy {
    fn id(&self) -> u64 {
        self.id
    }

    fn status(&self) -> ProxyStatus {
        self.status
    }

    fn connect(&mut self, _pkt: &VsockPacket, req: TsiConnectReq) -> ProxyUpdate {
        let mut update = ProxyUpdate::default();

        let result = match connect(
            self.fd,
            &SockAddr::Inet(InetAddr::new(IpAddr::V4(req.addr), req.port)),
        ) {
            Ok(()) => {
                debug!("vsock: connect: Connected");
                self.switch_to_connected();
                0
            }
            Err(nix::errno::Errno::EINPROGRESS) => {
                debug!("vsock: connect: Connecting");
                self.status = ProxyStatus::Connecting;
                0
            }
            Err(e) => {
                debug!("vsock: TcpProxy: Error connecting: {}", e);
                -nix::errno::errno()
            }
        };

        if self.status == ProxyStatus::Connecting {
            update.polling = Some((self.id, self.fd, EventSet::IN | EventSet::OUT));
        } else {
            if self.status == ProxyStatus::Connected {
                update.polling = Some((self.id, self.fd, EventSet::IN));
            }
            self.push_connect_rsp(result);
        }

        update
    }

    fn confirm_connect(&mut self, pkt: &VsockPacket) {
        debug!(
            "tcp: confirm_connect: local_port={} peer_port={}, src_port={}, dst_port={}",
            pkt.dst_port(),
            pkt.src_port(),
            self.local_port,
            self.peer_port,
        );

        self.peer_buf_alloc = pkt.buf_alloc();
        self.peer_fwd_cnt = Wrapping(pkt.fwd_cnt());

        self.local_port = pkt.dst_port();
        self.peer_port = pkt.src_port();

        // This response goes to the connection.
        let rx = MuxerRx::OpResponse {
            local_port: pkt.dst_port(),
            peer_port: pkt.src_port(),
        };
        push_packet(
            self.cid,
            rx,
            &self.rxq_stream,
            &self.queue_stream,
            &self.mem,
        );
    }

    fn getpeername(&mut self, pkt: &VsockPacket) {
        debug!("getpeername: id={}", self.id);

        let (result, addr, port) = match getpeername(self.fd) {
            Ok(name) => match name {
                SockAddr::Inet(iaddr) => match iaddr.ip() {
                    IpAddr::V4(ipv4) => (0, ipv4, iaddr.port()),
                    _ => (-libc::EINVAL, Ipv4Addr::new(0, 0, 0, 0), 0),
                },
                _ => (-libc::EINVAL, Ipv4Addr::new(0, 0, 0, 0), 0),
            },
            Err(e) => (-(e as i32), Ipv4Addr::new(0, 0, 0, 0), 0),
        };

        let data = TsiGetnameRsp { addr, port, result };

        debug!("getpeername: reply={:?}", data);

        // This response goes to the control port (DGRAM).
        let rx = MuxerRx::GetnameResponse {
            local_port: pkt.dst_port(),
            peer_port: pkt.src_port(),
            data,
        };
        push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);
    }

    fn sendmsg(&mut self, pkt: &VsockPacket) -> ProxyUpdate {
        debug!("vsock: tcp_proxy: sendmsg");

        let mut update = ProxyUpdate::default();

        let ret = if let Some(buf) = pkt.buf() {
            match send(self.fd, buf, MsgFlags::empty()) {
                Ok(sent) => {
                    if sent != buf.len() {
                        error!("couldn't set everything: buf={}, sent={}", buf.len(), sent);
                    }
                    self.tx_cnt += Wrapping(sent as u32);
                    sent as i32
                }
                Err(err) => -(err as i32),
            }
        } else {
            -libc::EINVAL
        };

        if ret > 0
            && (self.tx_cnt - self.last_tx_cnt_sent).0 as usize >= (defs::CONN_TX_BUF_SIZE / 2)
        {
            debug!(
                "sending credit update: id={}, tx_cnt={}, last_tx_cnt={}",
                self.id, self.tx_cnt, self.last_tx_cnt_sent
            );
            self.last_tx_cnt_sent = self.tx_cnt;
            // This packet goes to the connection.
            let rx = MuxerRx::CreditUpdate {
                local_port: pkt.dst_port(),
                peer_port: pkt.src_port(),
                fwd_cnt: self.tx_cnt.0,
            };
            push_packet(
                self.cid,
                rx,
                &self.rxq_stream,
                &self.queue_stream,
                &self.mem,
            );
            update.signal_queue = true;
        }

        debug!("vsock: tcp_proxy: sendmsg ret={}", ret);
        update
    }

    fn sendto_addr(&mut self, _req: TsiSendtoAddr) -> ProxyUpdate {
        ProxyUpdate::default()
    }

    fn listen(&mut self, pkt: &VsockPacket, req: TsiListenReq) -> ProxyUpdate {
        debug!(
            "listen: id={} addr={}, port={}, vm_port={} backlog={}",
            self.id, req.addr, req.port, req.vm_port, req.backlog
        );
        let mut update = ProxyUpdate::default();

        let result = if self.status == ProxyStatus::Listening {
            0
        } else {
            match bind(
                self.fd,
                &SockAddr::Inet(InetAddr::new(IpAddr::V4(req.addr), req.port)),
            ) {
                Ok(_) => {
                    debug!("tcp bind: id={}", self.id);
                    match listen(self.fd, req.backlog as usize) {
                        Ok(_) => {
                            debug!("tcp: proxy: id={}", self.id);
                            0
                        }
                        Err(e) => {
                            warn!("tcp: proxy: id={} err={}", self.id, e);
                            -(e as i32)
                        }
                    }
                }
                Err(e) => {
                    warn!("tcp bind: id={} err={}", self.id, e);
                    -(e as i32)
                }
            }
        };

        // This packet goes to the control port (DGRAM).
        let rx = MuxerRx::ListenResponse {
            local_port: pkt.dst_port(),
            peer_port: pkt.src_port(),
            result,
        };
        push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);

        if result == 0 {
            self.peer_port = req.vm_port;
            self.status = ProxyStatus::Listening;
            update.polling = Some((self.id, self.fd, EventSet::IN));
        }

        update
    }

    fn accept(&mut self, pkt: &VsockPacket, req: TsiAcceptReq) -> ProxyUpdate {
        debug!("accept: id={} flags={}", req.peer_port, req.flags);

        let mut update = ProxyUpdate::default();
        let result = match accept(self.fd) {
            Ok(accept_fd) => {
                update.new_proxy = Some((self.peer_port, accept_fd));
                0
            }
            Err(e) => -(e as i32),
        };

        if result != -libc::EAGAIN {
            // This packet goes to the control port (DGRAM).
            let rx = MuxerRx::AcceptResponse {
                local_port: pkt.dst_port(),
                peer_port: pkt.src_port(),
                result,
            };
            push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);
        }

        update
    }

    fn update_peer_credit(&mut self, pkt: &VsockPacket) -> ProxyUpdate {
        debug!(
            "update_credit: buf_alloc={} rx_cnt={} fwd_cnt={}",
            pkt.buf_alloc(),
            self.rx_cnt,
            pkt.fwd_cnt()
        );
        self.peer_buf_alloc = pkt.buf_alloc();
        self.peer_fwd_cnt = Wrapping(pkt.fwd_cnt());

        self.status = ProxyStatus::Connected;

        ProxyUpdate {
            polling: Some((self.id, self.fd, EventSet::IN)),
            ..Default::default()
        }
    }

    fn push_op_request(&self) {
        debug!(
            "push_op_request: id={}, local_port={} peer_port={}",
            self.id, self.local_port, self.peer_port
        );

        // This packet goes to the connection.
        let rx = MuxerRx::OpRequest {
            local_port: self.local_port,
            peer_port: self.peer_port,
        };
        push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);
    }

    fn process_op_response(&mut self, pkt: &VsockPacket) -> ProxyUpdate {
        debug!(
            "process_op_response: id={} src_port={} dst_port={}",
            self.id,
            pkt.src_port(),
            pkt.dst_port()
        );

        self.peer_buf_alloc = pkt.buf_alloc();
        self.peer_fwd_cnt = Wrapping(pkt.fwd_cnt());

        self.switch_to_connected();

        ProxyUpdate {
            polling: Some((self.id, self.fd, EventSet::IN)),
            push_accept: Some((self.id, self.parent_id)),
            ..Default::default()
        }
    }

    fn push_accept_rsp(&self, result: i32) {
        debug!(
            "push_accept_rsp: control_port: {}, result: {}",
            self.control_port, result
        );

        // This packet goes to the control port (DGRAM).
        let rx = MuxerRx::AcceptResponse {
            local_port: 1030,
            peer_port: self.control_port,
            result,
        };
        push_packet(self.cid, rx, &self.rxq_dgram, &self.queue_dgram, &self.mem);
    }

    fn shutdown(&mut self, pkt: &VsockPacket) {
        let recv_off = pkt.flags() & uapi::VSOCK_FLAGS_SHUTDOWN_RCV != 0;
        let send_off = pkt.flags() & uapi::VSOCK_FLAGS_SHUTDOWN_SEND != 0;

        let how = if recv_off && send_off {
            Shutdown::Both
        } else if recv_off {
            Shutdown::Read
        } else {
            Shutdown::Write
        };

        if let Err(e) = shutdown(self.fd, how) {
            warn!("error sending shutdown to socket: {}", e);
        }
    }

    fn release(&mut self) -> ProxyUpdate {
        debug!(
            "release: id={}, tx_cnt={}, last_tx_cnt={}",
            self.id, self.tx_cnt, self.last_tx_cnt_sent
        );
        ProxyUpdate {
            remove_proxy: true,
            ..Default::default()
        }
    }

    fn process_event(&mut self, evset: EventSet) -> ProxyUpdate {
        let mut update = ProxyUpdate::default();

        if evset.contains(EventSet::HANG_UP) {
            debug!("process_event: HANG_UP");
            if self.status == ProxyStatus::Connecting {
                self.push_connect_rsp(-libc::ECONNREFUSED);
            } else {
                self.push_reset();
            }

            self.status = ProxyStatus::Closed;
            update.polling = Some((self.id, self.fd, EventSet::empty()));
            update.signal_queue = true;
            update.remove_proxy = true;
            return update;
        }

        if evset.contains(EventSet::IN) {
            debug!("process_event: IN");
            if self.status == ProxyStatus::Connected {
                let (signal_queue, wait_credit) = self.recv_pkt();
                update.signal_queue = signal_queue;

                if wait_credit && self.status != ProxyStatus::WaitingCreditUpdate {
                    self.status = ProxyStatus::WaitingCreditUpdate;
                    let rx = MuxerRx::CreditRequest {
                        local_port: self.local_port,
                        peer_port: self.peer_port,
                        fwd_cnt: self.tx_cnt.0,
                    };
                    update.push_credit_req = Some(rx);
                }

                if self.status == ProxyStatus::Closed {
                    debug!(
                        "process_event: endpoint closed, sending reset: id={}",
                        self.id
                    );
                    self.push_reset();
                    update.signal_queue = true;
                    update.polling = Some((self.id(), self.fd, EventSet::empty()));
                    return update;
                } else if self.status == ProxyStatus::WaitingCreditUpdate {
                    debug!("process_event: WaitingCreditUpdate");
                    update.polling = Some((self.id(), self.fd, EventSet::empty()));
                }
            } else if self.status == ProxyStatus::Listening {
                match accept(self.fd) {
                    Ok(accept_fd) => {
                        update.new_proxy = Some((self.peer_port, accept_fd));
                    }
                    Err(e) => warn!("error accepting connection: id={}, err={}", self.id, e),
                };
                update.signal_queue = true;
                return update;
            } else {
                debug!(
                    "vsock::tcp: EventSet::IN while not connected: {:?}",
                    self.status
                );
            }
        }

        if evset.contains(EventSet::OUT) {
            debug!("process_event: OUT");
            if self.status == ProxyStatus::Connecting {
                self.switch_to_connected();
                self.push_connect_rsp(0);
                update.signal_queue = true;
                update.polling = Some((self.id(), self.fd, EventSet::IN));
            } else {
                error!("vsock::tcp: EventSet::OUT while not connecting");
            }
        }

        update
    }
}

impl AsRawFd for TcpProxy {
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for TcpProxy {
    fn drop(&mut self) {
        if let Err(e) = close(self.fd) {
            warn!("error closing proxy fd: {}", e);
        }
    }
}