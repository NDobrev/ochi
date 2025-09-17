# Bubble-sort demo: sort the 7-byte array in-place at address 0x42
# Initial array at 0x42: [2,4,5,6,3,2,1]

# Constants
mov d7, #7          ; n = 7
mov d4, #1          ; const 1

# Outer loop i = 0..n-2
mov d5, #0          ; i in d5 (avoid d0 in RR ops)
outer:
  ; if i >= n-1 goto done
  mov d1, #6        ; n-1 = 6
  jge.u d5, d1, done

  ; j = 0; ptr = 0x42
  mov d6, #0        ; j in d6
  lea a1, [a0+0x42]
  inner:
    ; limit = (n-1-i)
    mov d1, #6
    sub d1, d1, d5  ; limit = 6 - i
    ; if j >= limit, end inner
    jge.u d6, d1, end_inner

    ; load a[j] and a[j+1]
    ld.bu d2, [a1+0]
    lea a2, [a1+1]
    ld.bu d3, [a2+0]
    ; if d3 >= d2, no swap
    jge.u d3, d2, noswap
    ; swap
    st.b [a1+0], d3
    st.b [a2+0], d2
  noswap:
    ; advance j and ptr
    add d6, d6, d4
    lea a1, [a1+1]
    j inner
  end_inner:
  ; i++
  add d5, d5, d4
  j outer

done:
  ; Finished; stay here to avoid executing embedded data
  j done

; Data padding to ensure 0x42 resides within the image range.
; Place initial array at 0x42.
.word 0x00000000  ; 0x00
.word 0x00000000  ; 0x04
.word 0x00000000  ; 0x08
.word 0x00000000  ; 0x0C
.word 0x00000000  ; 0x10
.word 0x00000000  ; 0x14
.word 0x00000000  ; 0x18
.word 0x00000000  ; 0x1C
.word 0x00000000  ; 0x20
.word 0x00000000  ; 0x24
.word 0x00000000  ; 0x28
.word 0x00000000  ; 0x2C
.word 0x00000000  ; 0x30
.word 0x00000000  ; 0x34
.word 0x00000000  ; 0x38
.word 0x00000000  ; 0x3C
.byte 0x00        ; 0x40
.byte 0x00        ; 0x41
.byte 0x02        ; 0x42
.byte 0x04        ; 0x43
.byte 0x05        ; 0x44
.byte 0x06        ; 0x45
.byte 0x03        ; 0x46
.byte 0x02        ; 0x47
.byte 0x01        ; 0x48
