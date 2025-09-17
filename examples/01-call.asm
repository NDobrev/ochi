# Demonstrate CALL to a local label and fallthrough
movu d0, #1
call sub
mov d2, #2
j done
sub:
  mov d1, #3
done:
  .word 0xCAFEBABE

