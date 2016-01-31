\ vim:ts=3:sw=3:et:tw=80:ft=forth
\ Fun tests!

: testline ( size -- )
   0 do
     i .
   loop
;

: testbox ( size -- )
   dup 0 do
     dup 0 do
       [char] | emit j . i .
     loop
     cr
   loop
   drop
;
