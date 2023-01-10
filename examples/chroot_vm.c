// /*
//  * This is an example implementing chroot-like functionality with libkrun.
//  *
//  * It executes the requested command (relative to NEWROOT) inside a fresh
//  * Virtual Machine created and managed by libkrun.
//  */

// #include <errno.h>
// #include <stdio.h>
// #include <stdlib.h>
// #include <string.h>
// #include <unistd.h>
// #include <libkrun.h>

// #define MAX_ARGS_LEN 4096
// #ifndef MAX_PATH
// #define MAX_PATH 4096
// #endif

// int main(int argc, char *const argv[])
// {
//     char *const envp[] =
//     {
//         "TEST=works",
//         0
//     };
//     char *const port_map[] =
//     {
//         "18000:8000",
//         0
//     };
//     char *const rlimits[] =
//     {
//         // RLIMIT_NPROC = 6
//         "6=4096:8192",
//         0
//     };
//     char *mapped_volumes[2];
//     char current_path[MAX_PATH];
//     char volume_tail[] = ":/work\0";
//     char *volume;
//     int volume_len;
//     int ctx_id;
//     int err;
//     int i;

//     // if (argc < 3) {
//     //     printf("Invalid arguments\n");
//     //     printf("Usage: %s NEWROOT COMMAND [ARG...]\n", argv[0]);
//     //     return -1;
//     // }

//     // Set the log level to "off".
//     err = krun_set_log_level(3);
//     if (err) {
//         errno = -err;
//         perror("Error configuring log level");
//         return -1;
//     }

//     // Create the configuration context.
//     ctx_id = krun_create_ctx();
//     if (ctx_id < 0) {
//         errno = -err;
//         perror("Error creating configuration context");
//         return -1;
//     }

//     // Configure the number of vCPUs (1) and the amount of RAM (512 MiB).
//     if (err = krun_set_vm_config(ctx_id, 1, 1024)) {
//         errno = -err;
//         perror("Error configuring the number of vCPUs and/or the amount of RAM");
//         return -1;
//     }

//     // Use the first command line argument as the path to be used as root.
//     if (err = krun_set_root(ctx_id, "rootfs_IWASM")) {
//         errno = -err;
//         perror("Error configuring root path");
//         return -1;
//     }

//     if (getcwd(&current_path[0], MAX_PATH) == NULL) {
//         errno = -err;
//         perror("Error getting current directory");
//         return -1;
//     }

//     volume_len = strlen(current_path) + strlen(volume_tail) + 1;
//     volume = malloc(volume_len);
//     if (volume == NULL) {
//         errno = -err;
//         perror("Error allocating memory for volume string");
//     }

//     snprintf(volume, volume_len, "%s%s", current_path, volume_tail);
//     mapped_volumes[0] = volume;
//     mapped_volumes[1] = 0;

//     // Map "/tmp" as "/work" inside the VM.
//     if (err = krun_set_mapped_volumes(ctx_id, &mapped_volumes[0])) {
//         errno = -err;
//         perror("Error configuring mapped volumes");
//         return -1;
//     }

//     // Map port 18000 in the host to 8000 in the guest.
//     if (err = krun_set_port_map(ctx_id, &port_map[0])) {
//         errno = -err;
//         perror("Error configuring port map");
//         return -1;
//     }

//     // Configure the rlimits that will be set in the guest
//     if (err = krun_set_rlimits(ctx_id, &rlimits[0])) {
//         errno = -err;
//         perror("Error configuring rlimits");
//         return -1;
//     }

//     // Set the working directory to "/", just for the sake of completeness.
//     if (err = krun_set_workdir(ctx_id, "/")) {
//         errno = -err;
//         perror("Error configuring \"/\" as working directory");
//         return -1;
//     }

//     // Use the second argument as the path of the binary to be executed in the isolated
//     // context, relative to the root path.
//     if (err = krun_set_exec(ctx_id, "/iwasm", &argv[1], &envp[0])) {
//         errno = -err;
//         perror("Error configuring the parameters for the executable to be run");
//         return -1;
//     }

//     // Start and enter the microVM. Unless there is some error while creating the microVM
//     // this function never returns.
//     if (err = krun_start_enter(ctx_id)) {
//         errno = -err;
//         perror("Error creating the microVM");
//         return -1;
//     }

//     // Not reached.
//     return 0;
// }
/*************************************************************************
	> File Name: main.c
	> Author: Tianlei Xiong
	> Mail: qmyyxtl@sjtu.edu.cn 
	> Created Time: 2021年11月25日 星期四 13时26分18秒
 ************************************************************************/

#include<libkrun.h>
#include<unistd.h>
#include<stdio.h>
// #include<seccomp.h>

#include <errno.h>
//#include <stdio.h>
#include <stdlib.h>
#include <string.h>
//#include <unistd.h>
//#include <libkrun.h>

#define MAX_ARGS_LEN 4096
#ifndef MAX_PATH
#define MAX_PATH 4096
#endif
void runvm()
{
	char *const envp[] = {0};

	//arg
	char *const arg = "/hello.wasm";
	int ctx_id = krun_create_ctx();
	krun_set_vm_config(ctx_id,1,1024);
    	// krun_set_log_level(4);
	//the rootfs
	krun_set_root(ctx_id,"./rootfs_IWASM");

	/*
	 * first  arg is the ctx_id
	 * second arg is the path to executable, relative to the root
	 * third  arg is the arg should be passed as arguments to execute 
	 * 
	 *
	 * In this case, we set rootfs_fedora as root, and run iwasm in 
	 * the file /home/app with the argue "/home/app/hello.wasm"
	 * and after compiling and running, a string "hello" will be printed
	 */
	// krun_set_exec(ctx_id,"/home/iwasm",&arg,&envp[0]);
	krun_set_exec(ctx_id,"/iwasm",&arg,&envp[0]);
	krun_start_enter(ctx_id);
}

int main(int argc, char *const argv[]) {
    // runvm();
    

    char *const envp[] =
    {
        "TEST=works",
        0
    };
    char *const port_map[] =
    {
        "18000:8000",
        0
    };
    char *const rlimits[] =
    {
        // RLIMIT_NPROC = 6
        "6=4096:8192",
        0
    };
    char *mapped_volumes[2];
    char current_path[MAX_PATH];
    char volume_tail[] = ":/work\0";
    char *volume;
    int volume_len;
    int ctx_id;
    int err;
    int i;

    if (argc < 3) {
        printf("Invalid arguments\n");
        printf("Usage: %s NEWROOT COMMAND [ARG...]\n", argv[0]);
        return -1;
    }

    // Set the log level to "off".
    err = krun_set_log_level(0);
    if (err) {
        errno = -err;
        perror("Error configuring log level");
        return -1;
    }

    // Create the configuration context.
    ctx_id = krun_create_ctx();
    if (ctx_id < 0) {
        errno = -err;
        perror("Error creating configuration context");
        return -1;
    }

    // Configure the number of vCPUs (1) and the amount of RAM (512 MiB).
    if (err = krun_set_vm_config(ctx_id, 1, 4096)) {
        errno = -err;
        perror("Error configuring the number of vCPUs and/or the amount of RAM");
        return -1;
    }

    // Use the first command line argument as the path to be used as root.
    if (err = krun_set_root(ctx_id, argv[1])) {
        errno = -err;
        perror("Error configuring root path");
        return -1;
    }

    if (getcwd(&current_path[0], MAX_PATH) == NULL) {
        errno = -err;
        perror("Error getting current directory");
        return -1;
    }

    volume_len = strlen(current_path) + strlen(volume_tail) + 1;
    volume = malloc(volume_len);
    if (volume == NULL) {
        errno = -err;
        perror("Error allocating memory for volume string");
    }

    snprintf(volume, volume_len, "%s%s", current_path, volume_tail);
    mapped_volumes[0] = volume;
    mapped_volumes[1] = 0;

    // Map "/tmp" as "/work" inside the VM.
    if (err = krun_set_mapped_volumes(ctx_id, &mapped_volumes[0])) {
        errno = -err;
        perror("Error configuring mapped volumes");
        return -1;
    }

    // Map port 18000 in the host to 8000 in the guest.
    if (err = krun_set_port_map(ctx_id, &port_map[0])) {
        errno = -err;
        perror("Error configuring port map");
        return -1;
    }

    // Configure the rlimits that will be set in the guest
    if (err = krun_set_rlimits(ctx_id, &rlimits[0])) {
        errno = -err;
        perror("Error configuring rlimits");
        return -1;
    }

    // Set the working directory to "/", just for the sake of completeness.
    if (err = krun_set_workdir(ctx_id, "/")) {
        errno = -err;
        perror("Error configuring \"/\" as working directory");
        return -1;
    }

    // Use the second argument as the path of the binary to be executed in the isolated
    // context, relative to the root path.
    if (err = krun_set_exec(ctx_id, argv[2], &argv[3], &envp[0])) {
        errno = -err;
        perror("Error configuring the parameters for the executable to be run");
        return -1;
    }

    // Start and enter the microVM. Unless there is some error while creating the microVM
    // this function never returns.
    if (err = krun_start_enter(ctx_id)) {
        errno = -err;
        perror("Error creating the microVM");
        return -1;
    }

    // Not reached.
    return 0;	

    return 0;
}
