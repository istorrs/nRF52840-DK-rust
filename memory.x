/* Memory layout for nRF52840 - No SoftDevice (for testing) */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* Full flash and RAM available without SoftDevice */
  FLASH : ORIGIN = 0x00000000, LENGTH = 0x00100000  /* 1024K full flash */
  RAM : ORIGIN = 0x20000000, LENGTH = 0x00040000    /* 256K full RAM */
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);