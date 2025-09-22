/* Memory layout for nRF52840 with SoftDevice S140 v7.3.0 - OFFICIAL WORKING VERSION */

MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* NRF52840 with Softdevice S140 7.3.0 - optimized based on actual SoftDevice RAM usage */
  /* SoftDevice reports it needs RAM up to 0x200074c0 (29.2K), using 30K for safety margin */
  FLASH : ORIGIN = 0x00000000 + 156K, LENGTH = 1024K - 156K
  RAM : ORIGIN = 0x20000000 + 30K, LENGTH = 256K - 30K
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);