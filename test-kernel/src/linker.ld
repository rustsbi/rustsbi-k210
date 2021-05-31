OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80020000;

SECTIONS
{
    . = BASE_ADDRESS;
    start = .;

    .text : ALIGN(4K) {
        stext = .;
        *(.text.entry)
        *(.text .text.*)
        etext = .;
    }

    .rodata : ALIGN(4K) {
        srodata = .;
        *(.rodata .rodata.*)
        erodata = .;
    }

    .data : ALIGN(4K) {
        sidata = LOADADDR(.data);
        sdata = .;
        *(.data .data.*)
        edata = .;
    }

    .bss (NOLOAD) : ALIGN(4K)  {
        *(.bss.uninit)
        sbss = .;
        *(.sbss .bss .bss.*)
        ebss = .;
    }

    PROVIDE(end = .);
}
