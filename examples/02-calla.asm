# Absolute CALLA example (target EA is arbitrary here)
movu d0, #0x55AA
calla 0x00000100
mov d1, #4
.word 0x11111111

