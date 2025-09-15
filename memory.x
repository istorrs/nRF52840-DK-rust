/* Memory layout for nRF52840 with SoftDevice S140 */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52840 with Softdevice S140 7.0.1 */
  /* SoftDevice S140 takes up 0x26000 bytes (152K) of flash starting at 0x0 */
  FLASH : ORIGIN = 0x00026000, LENGTH = 0x000da000  /* 872K available for app */
  /* SoftDevice S140 uses 0x2540 bytes (9.3K) of RAM starting at 0x20000000 */
  RAM : ORIGIN = 0x20002540, LENGTH = 0x0003dac0    /* 246K available for app */
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);