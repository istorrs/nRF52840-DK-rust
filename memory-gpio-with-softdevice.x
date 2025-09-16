/* Memory layout for nRF52840 GPIO app that PRESERVES SoftDevice S140 v7.3.0 */
/* This allows GPIO and BLE apps to coexist without reflashing SoftDevice */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* nRF52840 with SoftDevice S140 7.3.0 PRESERVED */
  /* GPIO app starts AFTER SoftDevice to avoid overwriting it */
  FLASH : ORIGIN = 0x00000000 + 156K, LENGTH = 1024K - 156K
  RAM : ORIGIN = 0x20000000 + 31K, LENGTH = 256K - 31K
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);