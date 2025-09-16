/* Memory layout for nRF52840 WITHOUT SoftDevice - Full memory access */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* Full nRF52840 memory - no SoftDevice restrictions */
  FLASH : ORIGIN = 0x00000000, LENGTH = 0x00100000  /* Full 1MB flash */
  RAM : ORIGIN = 0x20000000, LENGTH = 0x00040000    /* Full 256K RAM */
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);