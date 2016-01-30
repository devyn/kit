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

: space ( -- ) bl emit ; \ emit space

\ Stack manipulation.
: -rot ( x y z -- z x y ) rot rot ;
: nip ( x y -- y ) swap drop ;
: tuck ( x y -- y x y ) swap over ;
: 2drop ( x y -- ) drop drop ;
: 2dup ( x y -- x y ) over over ;
: 2swap ( a b c d -- c d a b ) rot >r rot r> ;

\ Logic extensions.
: not ( flag -- !flag ) -1 xor ;
: 0= ( n -- flag ) 0 = ;

\ Arithmetic extensions.
: / ( n m -- n/m ) /mod nip ;
: mod ( n m -- n%m ) /mod drop ;
: 1+ ( n -- n+1 ) 1 + ;
: 1- ( n -- n-1 ) 1 - ;

\ Return stack manipulation.
: 2>r ( x y -- ) swap r> swap >r swap >r >r ;
: 2r> ( -- x y ) r> r> r> swap rot >r ;
: 2r@ ( -- x y ) r> 2r> over over 2>r rot >r ;
: r>drop ( -- ) r> r> drop >r ;
: 2r>drop ( -- ) r> 2r> drop drop >r ;

\ You shall not escape!
: exit ( -- ) postpone (ret) ; immediate

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
   0 postpone 2>r <mark
; immediate
: ?do ( limit index -- )
   postpone 2>r
   postpone 2r@
   postpone =
   postpone not
   postpone ?branch
   >mark
   <mark
; immediate
: (+=loop)
   r> swap r> + >r 2r@ = swap >r
;
: (+loop)
   postpone (+=loop)
   postpone ?branch
   <resolve
   dup if >resolve else drop then
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
   r> 2r> r@ -rot 2>r swap >r
;

: unloop ( -- ) r> 2r>drop >r ;

\ Allocation.

: char+ ( addr1 -- addr2 ) 1 + ;
: chars ( #chars -- #bytes ) ; \ chars are bytes; does nothing
: cell+ ( addr1 -- addr2 ) 8 + ;
: cells ( #cells -- #bytes ) 8 * ;

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

: , ( value -- )
   (here) @ aligned tuck ! cell+ (here) !
;
: c, ( char -- )
   (here) @ tuck c! char+ (here) !
;
: allot ( n -- )
   (here) @ + (here) !
;
: variable ( "<spaces>name" -- )
   create ,
;

\ System calls.
: exit-program ( status -- ) 1 0 syscall drop ;
: bye ( -- ) 0 exit-program ;

: type ( c-addr u -- ) 2 1 syscall drop ;

\ Strings.
: s" ( -- c-addr u )
   [char] " parse        \ get input string
   postpone (string)     \ (string) creates the string pointer at runtime
   dup cp,               \ write the length of the string
   cp swap               \ this is where we're going to put the string
   dup aligned 1 cells / \ number of cells we need to allocate for the string
   0 ?do 0 cp, loop      \ allocate them
   move                  \ put the string in
; immediate
: ." ( -- ) postpone s" postpone type ; immediate
: .( ( -- ) [char] ) parse type ; immediate

\ Compare two strings.
\ If the first string < the second string, returns -1.
\ If the first string > the second string, returns 1.
\ If they are equal, returns 0.
: compare ( c-addr1 u1 c-addr2 u2 -- n )
   rot >r >r
   begin
      \ make sure we haven't run out of string!
      2r@
      0 > swap
      0 > and
   while
      2dup c@ swap c@ swap
      2dup = not if
         2r>drop
         > if 1 else -1 then
         -rot 2drop
         exit
      else
         2drop
      then
      2r> 1- swap 1- swap 2>r
   repeat
   \ they must be equal up to this point, so go by difference in length
   2drop
   2r> 2dup = not if
      > if 1 else -1 then
   else
      2drop 0
   then
;

\ Archive utilities.
0 variable (system.kit) \ archive pointer
: system.kit ( -- addr )
   (system.kit) @ 0= if
      0 8 syscall \ SYSCALL_MMAP_ARCHIVE
      (system.kit) !
   then
   (system.kit) @
;
: archive-#entries ( -- n )
   system.kit cell+ @
;
: archive-entry0 ( -- addr )
   system.kit 2 cells +
;
: archive-next ( addr -- next-addr )
   2 cells + dup @ + cell+
;
: archive-entry.offset ( addr -- n )
   @
;
: archive-entry.length ( addr -- n )
   cell+ @
;
: archive-entry.name ( addr -- c-addr u )
   2 cells + dup @ swap cell+ swap
;
: archive-entry.body ( addr -- c-addr u )
   dup archive-entry.length >r
   archive-entry.offset system.kit +
   r>
;
: archive-scan ( c-addr u -- addr, 0 if not found )
   archive-entry0
   archive-#entries 0 do
      >r 2dup r@ archive-entry.name compare 0= if
         2drop r> unloop exit
      else
         r> archive-next
      then
   loop
   drop 2drop 0 \ not found
;
: ls ( -- )
   archive-entry0
   archive-#entries 0 do
      dup archive-entry.name cr type
      archive-next
   loop
   drop
;
: read ( c-addr u -- )
   archive-scan dup if archive-entry.body else 0 then
;

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

cr
27 emit .( [36m)
.( + kitFORTH ready) cr
27 emit .( [0m)
