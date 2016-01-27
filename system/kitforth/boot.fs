: cr 10 emit ;

: 2drop drop drop ;

: 2>r swap r> swap >r swap >r >r ;
: 2r> r> r> r> swap rot >r ;
: 2r@ r> 2r> over over 2>r rot >r ;
: r>drop r> r> drop >r ;
: 2r>drop r> 2r> drop drop >r ;

: not -1 xor ;
: literal (literal) ; immediate
: [char] char postpone literal ; immediate
: do postpone 2>r here ; immediate

: (+=loop)
  r> swap r> + >r 2r@ = swap >r ;

: (+loop)
  postpone (+=loop)
  (literal)
  postpone branch?
  postpone 2r>drop ;

: +loop (+loop) ; immediate
: loop 1 (literal) (+loop) ; immediate
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
