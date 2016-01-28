: \ 10 parse drop drop ; immediate

\ vim:ts=3:sw=3:et:tw=80:ft=forth
\ Now we can use comments! Hi there!
\ Let's define another type of comment.

: [char] char postpone literal ; immediate

: (
   [char] ) parse drop drop
; immediate

( Neat. We often use these comments for stack effects. )

: bl ( -- char ) 32 ; \ space character
: cr ( -- ) 10 emit ; \ emit newline

: not ( flag -- !flag ) -1 xor ;

: 2drop ( x y -- ) drop drop ;

\ Return stack manipulation.
: 2>r ( x y -- ) swap r> swap >r swap >r >r ;
: 2r> ( -- x y ) r> r> r> swap rot >r ;
: 2r@ ( -- x y ) r> 2r> over over 2>r rot >r ;
: r>drop ( -- ) r> r> drop >r ;
: 2r>drop ( -- ) r> 2r> drop drop >r ;

\ Allocation.

: here ( -- addr )
   (here) @ ;

: aligned ( addr -- addr-aligned )
   dup 7 and
   if cell+ then
   7 not and
;
: align ( -- )
   (here) @ aligned (here) !
;

: cell+ ( addr1 -- addr2 ) 8 + ;
: cells ( #cells -- #bytes ) 8 * ;

: , ( value -- )
   (here) @ aligned swap over ! cell+ (here) !
;
: allot ( n -- )
   (here) @ + (here) !
;
: variable ( "<spaces>name" -- )
   create ,
;

\ Backward MARK/RESOLVE. Use to BRANCH backward.
: <mark ( -- addr ) cp ;
: <resolve ( addr -- ) cp, ;

\ Forward MARK/RESOLVE. Use to BRANCH forward.
: >mark ( -- addr ) cp 0 cp, ;
: >resolve ( addr -- ) cp swap ! ;

\ flag IF true-code... THEN
\ flag IF true-code... ELSE false-code... THEN
: if ( flag -- )
   postpone ?branch >mark
; immediate
: else
   postpone branch >mark
   swap >resolve
; immediate
: then
   >resolve
; immediate

\ BEGIN code... flag UNTIL
  \ stops when flag is true
: begin <mark ; immediate
: until ( flag -- )
   postpone ?branch <resolve
; immediate

\ BEGIN code... flag WHILE true-code... REPEAT
  \ stops when flag is false
: while ( flag -- ) postpone if ; immediate
: repeat
   postpone branch
   swap <resolve
   postpone then ; immediate

\ limit index DO code... LOOP
\ limit index DO code... n +LOOP

: do ( limit index -- )
   postpone 2>r <mark
; immediate
: (+=loop)
   r> swap r> + >r 2r@ = swap >r
;
: (+loop)
   postpone (+=loop)
   postpone ?branch
   <resolve
   postpone 2r>drop
;
: +loop
   (+loop)
; immediate
: loop
   1 postpone literal (+loop)
; immediate

: i ( -- i ) \ inner loop index
   r> r@ swap >r
;
: j ( -- j ) \ outer loop index
   r> 2r> r@ rot rot 2>r swap >r
;

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
