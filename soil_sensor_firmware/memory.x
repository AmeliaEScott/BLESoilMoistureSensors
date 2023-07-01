MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* These values correspond to the NRF52840 with Softdevices S140 7.0.1 */
  /* FLASH : ORIGIN = 0x00000000 + 100k, LENGTH = 192k - 92k */
  /* RAM : ORIGIN = 0x20000000 + 0x2f38, LENGTH = 24k - 0x2f38 */
  FLASH : ORIGIN = 0x00000000, LENGTH = 192k
  RAM : ORIGIN = 0x20000000, LENGTH = 24k
}