/* Memory layout for nRF52840 with SoftDevice S140 v7.3.0 */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* SoftDevice S140 v7.3.0 takes up 160KB (0x27000) of flash starting at 0x0 */
  FLASH : ORIGIN = 0x00027000, LENGTH = 0x000d9000  /* 868K available for app */
  /* SoftDevice S140 v7.3.0 uses 128KB (0x20000) of RAM starting at 0x20000000 */
  RAM : ORIGIN = 0x20020000, LENGTH = 0x00020000    /* 128K available for app */
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);