ENTRY(kernel_entry)

/*
 * FORMAT/ARCH
 *
 * It's hard to find the right names for this.
 * Keyword: "bfdname" in GNU LD's source code.
 */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH("i386:x86-64")

/* Program headers for the segments. */
PHDRS
{
    /* FLAGS: 0b111 => +read +write +execute */
    rx     PT_LOAD FLAGS(5);
    ro     PT_LOAD FLAGS(4);
    rw     PT_LOAD FLAGS(6);
    note   PT_NOTE;
}

/* Base address for high-level code linking. */
LINK_ADDR = 0xffffffff88200000;

/*
 * Ensure that all LOAD segments are continuous in memory. This simplifies
 * setting-up page tables.
 */
LINK_ADDR_RX = LINK_ADDR;
LINK_ADDR_RO = ALIGN(LINK_ADDR_RX + SIZEOF(.text), 2M);
LINK_ADDR_RW = ALIGN(LINK_ADDR_RO + SIZEOF(.rodata), 2M);

SECTIONS {
    /*
     * All symbols from the high-level Rust code.
     */
    .text LINK_ADDR_RX :
    {
        *(.text.entry)
        *(.text .text.*)
    } : rx

    .rodata LINK_ADDR_RO :
    {
        *(.rodata .rodata.*)
    } : ro

    .data LINK_ADDR_RW : /* Link + Load Addr auto increment */
    {
        *(.data .data.*)
    } : rw

    .bss :
    {
        *(COMMON)
        *(.bss .bss.*)
    } : rw

    .note : {
        *(.note .note.*)
    } : note

    /*
     * General Assertions
     */
    ASSERT(ALIGN(LINK_ADDR, 2M) == LINK_ADDR, "LINK_ADDR must be 2 MiB aligned")

    /DISCARD/ :
    {
        *(.comment .comment.*)
        *(.dynamic)
        *(.eh_frame*)
        *(.got .got.*)
        *(.note.*)
        *(.plt .plt.*)
        *(.rela .rela.*)
    }

}
