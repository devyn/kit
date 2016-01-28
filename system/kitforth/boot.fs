: cr 10 emit ;

: 2drop drop drop ;

: 2>r swap r> swap >r swap >r >r ;
: 2r> r> r> r> swap rot >r ;
: 2r@ r> 2r> over over 2>r rot >r ;
: r>drop r> r> drop >r ;
: 2r>drop r> 2r> drop drop >r ;

: <mark here ;
: <resolve here+ ! ;
: >mark here+ dup 0 swap ! ;
: >resolve here swap ! ;

: not -1 xor ;
: [char] char postpone literal ; immediate

: if postpone ?branch >mark ; immediate
: else postpone branch >mark
       swap >resolve ; immediate
: then >resolve ; immediate

: begin <mark ; immediate
: until postpone ?branch <resolve ; immediate
: while postpone if ; immediate
: repeat postpone branch
         swap <resolve
         postpone then ; immediate

: do postpone 2>r <mark ; immediate

: (+=loop)
  r> swap r> + >r 2r@ = swap >r ;

: (+loop)
  postpone (+=loop)
  postpone ?branch
  <resolve
  postpone 2r>drop ;

: +loop (+loop) ; immediate
: loop 1 postpone literal (+loop) ; immediate

: i r> r@ swap >r ;
: j r> 2r> r@ rot rot 2>r swap >r ;

: testline
  0 do
    i .
  loop ;

: testbox
  dup 0 do
    dup 0 do
      [char] | emit j . i .
    loop
    cr
  loop
  drop ;
