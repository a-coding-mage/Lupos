/* SPDX-License-Identifier: GPL-2.0 */
/*
 * Minimal early userspace for the canonical disk-root image.
 *
 * Linux source/ABI references:
 *   vendor/linux/init/main.c          - /init suppresses prepare_namespace()
 *   vendor/linux/init/do_mounts.c     - root=, rootfstype=, rootflags=, ro/rw
 *   vendor/linux/include/uapi/linux/module.h - finit_module(2) flags
 *   util-linux sys-utils/switch_root.c - MS_MOVE + chroot + exec handoff
 *
 * This is deliberately userspace.  Vendor Linux never reads /etc/modules in
 * the kernel: an initramfs with modular root-storage drivers must provide an
 * executable /init which loads them before mounting and switching to the real
 * root.  The program is a freestanding relocation-free static PIE and uses
 * only the x86-64 Linux syscall ABI, so the initramfs does not need a
 * host-dependent libc or shell closure.
 */

typedef unsigned long usize;
typedef long isize;

#define SYS_read 0
#define SYS_write 1
#define SYS_open 2
#define SYS_close 3
#define SYS_pread64 17
#define SYS_nanosleep 35
#define SYS_execve 59
#define SYS_chdir 80
#define SYS_mkdir 83
#define SYS_chroot 161
#define SYS_mount 165
#define SYS_finit_module 313
#define SYS_exit 60

#define O_RDONLY 0
#define O_CLOEXEC 02000000
#define MS_RDONLY 1
#define MS_MOVE 8192
#define EEXIST 17

struct timespec {
	isize tv_sec;
	isize tv_nsec;
};

static inline isize syscall6(isize nr, isize a1, isize a2, isize a3,
			     isize a4, isize a5, isize a6)
{
	register isize r10 __asm__("r10") = a4;
	register isize r8 __asm__("r8") = a5;
	register isize r9 __asm__("r9") = a6;
	isize ret;

	__asm__ volatile("syscall"
			 : "=a"(ret)
			 : "a"(nr), "D"(a1), "S"(a2), "d"(a3),
			   "r"(r10), "r"(r8), "r"(r9)
			 : "rcx", "r11", "memory");
	return ret;
}

static inline isize syscall5(isize nr, isize a1, isize a2, isize a3,
			     isize a4, isize a5)
{
	return syscall6(nr, a1, a2, a3, a4, a5, 0);
}

static inline isize syscall4(isize nr, isize a1, isize a2, isize a3,
			     isize a4)
{
	return syscall6(nr, a1, a2, a3, a4, 0, 0);
}

static inline isize syscall3(isize nr, isize a1, isize a2, isize a3)
{
	return syscall6(nr, a1, a2, a3, 0, 0, 0);
}

static inline isize syscall2(isize nr, isize a1, isize a2)
{
	return syscall6(nr, a1, a2, 0, 0, 0, 0);
}

static inline isize syscall1(isize nr, isize a1)
{
	return syscall6(nr, a1, 0, 0, 0, 0, 0);
}

static usize string_length(const char *text)
{
	usize length = 0;

	while (text[length])
		length++;
	return length;
}

static int string_equal(const char *left, const char *right)
{
	usize index = 0;

	while (left[index] && right[index]) {
		if (left[index] != right[index])
			return 0;
		index++;
	}
	return left[index] == right[index];
}

static int starts_with(const char *text, const char *prefix)
{
	usize index = 0;

	while (prefix[index]) {
		if (text[index] != prefix[index])
			return 0;
		index++;
	}
	return 1;
}

static void copy_string(char *destination, usize capacity, const char *source)
{
	usize index = 0;

	if (!capacity)
		return;
	while (source[index] && index + 1 < capacity) {
		destination[index] = source[index];
		index++;
	}
	destination[index] = 0;
}

static void write_all(int fd, const char *data, usize length)
{
	while (length) {
		isize written = syscall3(SYS_write, fd, (isize)data, length);

		if (written <= 0)
			return;
		data += written;
		length -= (usize)written;
	}
}

static void write_text(const char *text)
{
	write_all(2, text, string_length(text));
}

static void write_number(isize value)
{
	char digits[32];
	usize count = 0;
	unsigned long magnitude;

	if (value < 0) {
		write_all(2, "-", 1);
		magnitude = (unsigned long)(-(value + 1)) + 1;
	} else {
		magnitude = (unsigned long)value;
	}
	if (!magnitude) {
		write_all(2, "0", 1);
		return;
	}
	while (magnitude) {
		digits[count++] = (char)('0' + magnitude % 10);
		magnitude /= 10;
	}
	while (count) {
		count--;
		write_all(2, &digits[count], 1);
	}
}

static void report_error(const char *operation, isize result)
{
	write_text("initramfs: ");
	write_text(operation);
	write_text(" failed: errno ");
	write_number(result < 0 ? -result : result);
	write_text("\n");
}

static __attribute__((noreturn)) void terminate(isize status)
{
	syscall1(SYS_exit, status);
	for (;;)
		__asm__ volatile("pause");
}

static __attribute__((noreturn)) void fatal(const char *operation, isize result)
{
	report_error(operation, result);
	terminate(1);
}

static isize open_readonly(const char *path)
{
	return syscall3(SYS_open, (isize)path, O_RDONLY | O_CLOEXEC, 0);
}

static isize read_small_file(const char *path, char *buffer, usize capacity)
{
	isize fd;
	isize total = 0;

	if (!capacity)
		return -1;
	fd = open_readonly(path);
	if (fd < 0)
		return fd;
	while ((usize)total + 1 < capacity) {
		isize count = syscall3(SYS_read, fd, (isize)(buffer + total),
				       capacity - (usize)total - 1);

		if (count < 0) {
			syscall1(SYS_close, fd);
			return count;
		}
		if (!count)
			break;
		total += count;
	}
	syscall1(SYS_close, fd);
	buffer[total] = 0;
	return total;
}

static void make_directory(const char *path, unsigned int mode)
{
	isize result = syscall2(SYS_mkdir, (isize)path, mode);

	if (result < 0 && result != -EEXIST)
		fatal(path, result);
}

static void mount_early_filesystem(const char *source, const char *target,
				   const char *type, const char *data)
{
	isize result = syscall5(SYS_mount, (isize)source, (isize)target,
				 (isize)type, 0, (isize)data);

	/* Lupos may already have mounted the Linux early pseudo filesystems. */
	if (result < 0 && result != -16)
		report_error(target, result);
}

static int device_exists(const char *path);

static int root_storage_device_exists(void)
{
	return device_exists("/dev/vda") || device_exists("/dev/sda") ||
	       device_exists("/dev/hda") || device_exists("/dev/nvme0n1");
}

static void load_early_modules(void)
{
	char paths[8192];
	isize length = read_small_file("/etc/initramfs.modules", paths,
				       sizeof(paths));
	isize offset = 0;

	if (length < 0)
		fatal("open /etc/initramfs.modules", length);
	/* Linux initramfs generators retain the generic root-driver closure, while
	 * udev/modprobe only loads transports needed by the discovered hardware.
	 * The freestanding loader has the same observable policy: stop once a live
	 * root-storage gendisk has appeared instead of probing unrelated chains. */
	if (root_storage_device_exists())
		return;
	while (offset < length) {
		char *path = &paths[offset];
		isize end = offset;
		isize fd;
		isize result;

		while (end < length && paths[end] != '\n')
			end++;
		paths[end] = 0;
		offset = end + 1;
		if (!path[0] || path[0] == '#')
			continue;

		write_text("initramfs: loading ");
		write_text(path);
		write_text("\n");
		fd = open_readonly(path);
		if (fd < 0) {
			report_error(path, fd);
			continue;
		}
		result = syscall3(SYS_finit_module, fd, (isize)"", 0);
		syscall1(SYS_close, fd);
		/* EEXIST is successful dependency reuse, matching modprobe. */
		if (result < 0 && result != -EEXIST)
			report_error(path, result);
		else if (root_storage_device_exists())
			return;
	}
}

static void parse_command_line(char *root, usize root_capacity,
			       char *root_type, usize type_capacity,
			       char *root_flags, usize flags_capacity,
			       unsigned long *mount_flags)
{
	char command_line[4096];
	isize length = read_small_file("/proc/cmdline", command_line,
				       sizeof(command_line));
	isize offset = 0;

	copy_string(root, root_capacity, "LABEL=lupos-root");
	copy_string(root_type, type_capacity, "ext4");
	root_flags[0] = 0;
	*mount_flags = MS_RDONLY;
	if (length < 0)
		return;
	while (offset < length) {
		char *token;
		isize end;

		while (offset < length &&
		       (command_line[offset] == ' ' || command_line[offset] == '\n' ||
			command_line[offset] == '\t'))
			offset++;
		if (offset >= length)
			break;
		token = &command_line[offset];
		end = offset;
		while (end < length && command_line[end] != ' ' &&
		       command_line[end] != '\n' && command_line[end] != '\t')
			end++;
		command_line[end] = 0;
		offset = end + 1;

		if (starts_with(token, "root="))
			copy_string(root, root_capacity, token + 5);
		else if (starts_with(token, "rootfstype="))
			copy_string(root_type, type_capacity, token + 11);
		else if (starts_with(token, "rootflags="))
			copy_string(root_flags, flags_capacity, token + 10);
		else if (string_equal(token, "rw"))
			*mount_flags &= ~MS_RDONLY;
		else if (string_equal(token, "ro"))
			*mount_flags |= MS_RDONLY;
	}
}

static int ext4_label_matches(const char *device, const char *wanted)
{
	unsigned char superblock[1024];
	isize fd = open_readonly(device);
	isize count;
	usize index;

	if (fd < 0)
		return 0;
	count = syscall4(SYS_pread64, fd, (isize)superblock,
			  sizeof(superblock), 1024);
	syscall1(SYS_close, fd);
	if (count < 136 || superblock[0x38] != 0x53 ||
	    superblock[0x39] != 0xef)
		return 0;
	for (index = 0; index < 16; index++) {
		unsigned char actual = superblock[0x78 + index];
		unsigned char expected = (unsigned char)wanted[index];

		if (actual != expected)
			return 0;
		if (!expected)
			return 1;
	}
	return wanted[16] == 0;
}

static int device_exists(const char *path)
{
	isize fd = open_readonly(path);

	if (fd < 0)
		return 0;
	syscall1(SYS_close, fd);
	return 1;
}

static int resolve_root_device(const char *root, char *resolved, usize capacity)
{
	if (starts_with(root, "/dev/")) {
		if (!device_exists(root))
			return 0;
		copy_string(resolved, capacity, root);
		return 1;
	}
	if (!starts_with(root, "LABEL="))
		return 0;
	/* Keep these as direct literals: a freestanding static PIE must not need
	 * runtime R_X86_64_RELATIVE processing before its first syscall. */
#define TRY_LABEL_DEVICE(device)                                                \
	do {                                                                      \
		if (ext4_label_matches((device), root + 6)) {                       \
			copy_string(resolved, capacity, (device));                    \
			return 1;                                                       \
		}                                                                     \
	} while (0)
	TRY_LABEL_DEVICE("/dev/vda");
	TRY_LABEL_DEVICE("/dev/sda");
	TRY_LABEL_DEVICE("/dev/hda");
	TRY_LABEL_DEVICE("/dev/nvme0n1");
	TRY_LABEL_DEVICE("/dev/vda1");
	TRY_LABEL_DEVICE("/dev/sda1");
#undef TRY_LABEL_DEVICE
	return 0;
}

static void wait_for_root_device(const char *root, char *resolved, usize capacity)
{
	struct timespec delay = { 0, 100000000 };
	unsigned int attempt;

	write_text("initramfs: waiting for root ");
	write_text(root);
	write_text("\n");
	for (attempt = 0; attempt < 600; attempt++) {
		if (resolve_root_device(root, resolved, capacity))
			return;
		syscall2(SYS_nanosleep, (isize)&delay, 0);
	}
	fatal("root device timeout", -19);
}

static void move_mount(const char *source, const char *target)
{
	isize result = syscall5(SYS_mount, (isize)source, (isize)target, 0,
				 MS_MOVE, 0);

	if (result < 0)
		fatal(target, result);
}

static __attribute__((noreturn)) void run_init(void)
{
	char root[256];
	char root_type[64];
	char root_flags[512];
	char root_device[256];
	unsigned long mount_flags;
	isize result;
	char *const argv[] = { (char *)"/sbin/init", 0 };
	char *const envp[] = { (char *)"HOME=/", (char *)"TERM=linux", 0 };

	make_directory("/dev", 0755);
	make_directory("/proc", 0555);
	make_directory("/sys", 0555);
	make_directory("/run", 0755);
	make_directory("/new_root", 0755);
	mount_early_filesystem("devtmpfs", "/dev", "devtmpfs", "mode=0755");
	mount_early_filesystem("proc", "/proc", "proc", "");
	mount_early_filesystem("sysfs", "/sys", "sysfs", "");
	mount_early_filesystem("tmpfs", "/run", "tmpfs", "mode=0755");

	load_early_modules();
	parse_command_line(root, sizeof(root), root_type, sizeof(root_type),
			   root_flags, sizeof(root_flags), &mount_flags);
	wait_for_root_device(root, root_device, sizeof(root_device));

	write_text("initramfs: mounting ");
	write_text(root_device);
	write_text(" on /new_root\n");
	result = syscall5(SYS_mount, (isize)root_device, (isize)"/new_root",
			  (isize)root_type, mount_flags,
			  root_flags[0] ? (isize)root_flags : 0);
	if (result < 0)
		fatal("mount real root", result);

	move_mount("/dev", "/new_root/dev");
	move_mount("/proc", "/new_root/proc");
	move_mount("/sys", "/new_root/sys");
	move_mount("/run", "/new_root/run");
	result = syscall1(SYS_chdir, (isize)"/new_root");
	if (result < 0)
		fatal("chdir new root", result);
	/* util-linux switch_root(8): move the new mount onto /, then chroot. */
	move_mount(".", "/");
	result = syscall1(SYS_chroot, (isize)".");
	if (result < 0)
		fatal("chroot", result);
	result = syscall1(SYS_chdir, (isize)"/");
	if (result < 0)
		fatal("chdir /", result);

	write_text("initramfs: running /sbin/init\n");
	result = syscall3(SYS_execve, (isize)"/sbin/init", (isize)argv,
			  (isize)envp);
	fatal("exec /sbin/init", result);
}

__attribute__((noreturn)) void _start(void)
{
	run_init();
}
