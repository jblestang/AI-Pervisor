/* Multiboot1 (a.out/ADDR style) + 32→64 trampoline for QEMU `-kernel`.
 *
 * QEMU rejects ELF64 Multiboot (`Cannot load x86-64 image`). We therefore:
 *   1. Embed Multiboot1 with MULTIBOOT_HEADER_HAS_ADDR (flags bit 16)
 *   2. Have xtask `objcopy -O binary` the ELF and pass the .bin to `-kernel`
 */
.section .multiboot, "a"
.align 4
.global mb1_header
mb1_header:
    .long 0x1BADB002              /* magic */
    .long 0x00010000              /* flags: MULTIBOOT_HEADER_HAS_ADDR */
    .long -(0x1BADB002 + 0x00010000) /* checksum */
    .long mb1_header              /* header_addr */
    .long __load_start            /* load_addr */
    .long __load_end              /* load_end_addr */
    .long __bss_end               /* bss_end_addr */
    .long _start                  /* entry_addr */

.section .boot.bss, "aw", @nobits
.align 4096
boot_pml4:
    .skip 4096
boot_pdpt:
    .skip 4096
boot_pd:
    .skip 4096
.align 16
boot_stack:
    .skip 65536
boot_stack_top:

.section .text.boot, "ax"
.code32
.global _start
.type _start, @function
_start:
    cli
    /* Multiboot1: eax = magic, ebx = info. Preserve ebx. */
    mov %ebx, %esi

    /* QEMU -debugcon (port 0xE9): prove entry before paging. */
    mov $0xE9, %dx
    mov $'H', %al
    out %al, %dx
    mov $'V', %al
    out %al, %dx

    /* Identity-map first 1 GiB with 2 MiB PAE pages. */
    mov $boot_pml4, %eax
    mov $boot_pdpt, %ebx
    or $0x3, %ebx
    mov %ebx, (%eax)

    mov $boot_pdpt, %eax
    mov $boot_pd, %ebx
    or $0x3, %ebx
    mov %ebx, (%eax)

    mov $boot_pd, %edi
    mov $0x83, %eax
    xor %ebx, %ebx
    mov $512, %ecx
1:
    movl %eax, (%edi)
    movl %ebx, 4(%edi)
    add $8, %edi
    add $0x200000, %eax
    adc $0, %ebx
    loop 1b

    lgdt boot_gdt_desc

    mov %cr4, %eax
    or $(1 << 5), %eax
    mov %eax, %cr4

    mov $boot_pml4, %eax
    mov %eax, %cr3

    mov $0xC0000080, %ecx
    rdmsr
    or $(1 << 8), %eax
    wrmsr

    mov %cr0, %eax
    or $(1 << 31), %eax
    or $1, %eax
    mov %eax, %cr0

    ljmp $0x08, $long_mode_entry

.code64
long_mode_entry:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %ss
    mov %ax, %fs
    mov %ax, %gs
    lea boot_stack_top(%rip), %rsp

    mov %esi, %edi
    call rust_entry

1:
    hlt
    jmp 1b

.align 8
boot_gdt:
    .quad 0
    .quad 0x00AF9A000000FFFF
    .quad 0x00CF92000000FFFF
boot_gdt_desc:
    .word boot_gdt_desc - boot_gdt - 1
    .long boot_gdt
