# Basic program: MOVs, jump, and data
movu d0, #0x1234
mov d1, #7
start:
  call next
  j end
next:
  mov d2, #0
end:
  .word 0xDEADBEEF
  .byte 0xAA

