# Cases for 07-matching-and-ranking.md

Conventions for this file (the case grammar is the one of `docs/specs/README.md`):

- `match "<p>"` runs the matcher with the mode of the `settings:` line on every line of the `before` block. The `after` block has
  one line per name, in the same order: the name with the matched fragments in square brackets, then `=>` and the degree, or `=> no match`.
  Brackets are display only; the fragments are the half-open ranges described in section 1 of the spec.
- `Go to Class, type "<p>"` builds the name list of a Go to Class tab for the typed text (spec section 5): the `before` block is the set of
  candidate names, the `after` block is the **ordered** result list, one line per entry: `name`, the highlighted fragments, the final weight
  (matching degree plus the prefer-start bonus of 10000 where it applies, plus 5000 for an exact qualified match).
  Names with equal weight are ordered by name ignoring case.
- `Go to File, type "<p>"`: the `before` block is a list of project-relative file paths; the `after` block is the displayed order (descending weight;
  equal weights keep the order of the tier rules of SRCH-MAT-046) with the weight of each path.
- Degrees were computed with the reference algorithm of the spec; implementations must reproduce them exactly.

### SRCH-MAT-001 case 1: space and star are wildcards between fragments
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
ReplacePathToMacroMap
ReplaceMap
MapRepl
```
keys: match "repl map"
```after
[Repl]acePathToMacro[Map]  =>  961
[Repl]ace[Map]  =>  991
MapRepl  =>  no match
```

### SRCH-MAT-001 case 2: a leading star lifts the start anchor
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
barFoo
foobar
bar
```
keys: match "*foo"
```after
bar[Foo]  =>  1001
[foo]bar  =>  1003
bar  =>  no match
```

### SRCH-MAT-001 case 3: a trailing star-space is dropped
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
foobar
barfoo
```
keys: match "foo* "
```after
[foo]bar  =>  1003
barfoo  =>  no match
```

### SRCH-MAT-001 case 4: a leading space requires a word start for the first letter
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
fbi
foo Bar
fooBar
```
keys: match " b"
```after
fbi  =>  no match
foo [B]ar  =>  1000
foo[B]ar  =>  1000
```

### SRCH-MAT-002 case 1: an all-lowercase pattern may skip to a later hump
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
Hashmap
HAMAP
```
keys: match "hamap"
```after
[Ha]sh[Map]  =>  991
Hashmap  =>  no match
[HAMAP]  =>  1002
```

### SRCH-MAT-002 case 2: a leading capital does not make the pattern mixed
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
Hashmap
```
keys: match "Hamap"
```after
[Ha]sh[Map]  =>  1042
Hashmap  =>  no match
```

### SRCH-MAT-002 case 3: a capital after a lowercase letter makes it mixed
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
hashMap
Hashmap
```
keys: match "haMap"
```after
[Ha]sh[Map]  =>  1051
[ha]sh[Map]  =>  1053
Hashmap  =>  no match
```

### SRCH-MAT-003 case 1: names without the pattern characters in order are rejected
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
NameUtilTest
NextXt
TextN
```
keys: match "nxt"
```after
NameUtilTest  =>  no match
[N]ext[Xt]  =>  991
TextN  =>  no match
```

### SRCH-MAT-010 case 1: uppercase letters pick hump starts
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
NameUtilTest
NUMBER
nameUtil
```
keys: match "NU"
```after
[N]ame[U]tilTest  =>  1101
[NU]MBER  =>  1102
[n]ame[U]til  =>  1050
```

### SRCH-MAT-010 case 2: longest fragment first, then humps
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
NameUtilTest
NamUserTest
```
keys: match "NaUT"
```after
[Na]me[U]til[T]est  =>  1150
[Na]m[U]ser[T]est  =>  1150
```

### SRCH-MAT-010 case 3: a wrong letter after a hump start fails
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
NameUtilTest
```
keys: match "NeUT"
```after
NameUtilTest  =>  no match
```

### SRCH-MAT-011 case 1: without a leading wildcard the first fragment starts at index 0
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
nameUtilTest
anameUtilTest
```
keys: match "nu"
```after
[n]ame[U]tilTest  =>  992
anameUtilTest  =>  no match
```

### SRCH-MAT-011 case 2: a leading star lifts the anchor
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
anameUtilTest
nameUtilTest
```
keys: match "*nu"
```after
anameUtilTest  =>  no match
[n]ame[U]tilTest  =>  992
```

### SRCH-MAT-012 case 1: letters skip to word starts, not into words
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
NameUtilTest
Nettest
NameUtiltest
```
keys: match "nt"
```after
[N]ameUtil[T]est  =>  980
Nettest  =>  no match
NameUtiltest  =>  no match
```

### SRCH-MAT-012 case 2: a separator in the pattern allows any position after it
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
ORGS_POSITION_ACCOUNTABILITY
ORGS_ACCOUNT
ORGSACC
```
keys: match "ORGS_ACC"
```after
[ORGS_]POSITION_[ACC]OUNTABILITY  =>  1332
[ORGS_ACC]OUNT  =>  1353
ORGSACC  =>  no match
```

### SRCH-MAT-012 case 3: hyphen and underscore separated names count as words
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
base-layout-without-navigation.xhtml
base_layout_without_navigation.xhtml
baseLayoutWithoutNavigation
```
keys: match "BLWN"
```after
[b]ase-[l]ayout-[w]ithout-[n]avigation.xhtml  =>  965
[b]ase_[l]ayout_[w]ithout_[n]avigation.xhtml  =>  965
[b]ase[L]ayout[W]ithout[N]avigation  =>  1148
```

### SRCH-MAT-013 case 1: an uppercase letter lands only on a word start when skipping
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
Hashmap
```
keys: match "HM"
```after
[H]ash[M]ap  =>  1101
Hashmap  =>  no match
```

### SRCH-MAT-013 case 2: after a leading star an uppercase letter may hit a lowercase letter
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
hashmap
hashMap
```
keys: match "*M"
```after
hash[m]ap  =>  1
hash[M]ap  =>  1051
```

### SRCH-MAT-013 case 3: a mixed pattern does not allow it
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
hashmap
hashMap
```
keys: match "*aM"
```after
hashmap  =>  no match
hashMap  =>  no match
```

### SRCH-MAT-013 case 4: match-case mode keeps the case
settings: mode=MATCH_CASE, prefer-start=off
language: plain
```before
hashmap
hashMap
```
keys: match "*M"
```after
hashmap  =>  no match
hash[M]ap  =>  1051
```

### SRCH-MAT-013 case 5: first-letter mode compares only the first letter
settings: mode=FIRST_LETTER, prefer-start=off
language: plain
```before
FooBar
fooBar
Foobar
```
keys: match "Foo"
```after
[Foo]Bar  =>  1052
fooBar  =>  no match
[Foo]bar  =>  1052
```

### SRCH-MAT-014 case 1: a lowercase letter of a mixed pattern cannot skip to a hump
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
hashMap
hashmap
HashMap
```
keys: match "hM"
```after
[h]ash[M]ap  =>  1052
hashmap  =>  no match
[H]ash[M]ap  =>  1050
```

### SRCH-MAT-016 case 1: a pattern dot passes name dots only through the dot itself
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
foo.bar.goo
foo.bar.gXoo
```
keys: match "foo.goo"
```after
[foo.]bar.[goo]  =>  974
foo.bar.gXoo  =>  no match
```

### SRCH-MAT-016 case 2: a star-dot pattern does not cross dots
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
sm.th.iks.concierge
sm.ico
```
keys: match "*.ico"
```after
sm.th.iks.concierge  =>  no match
sm[.ico]  =>  4
```

### SRCH-MAT-016 case 3: an extension anchors the end
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
activity_report_summary.jsp
activity_report_summary.xml
activity_report_summary_justsometingwrong.xml
```
keys: match "ARS.j"
```after
[a]ctivity_[r]eport_[s]ummary[.j]sp  =>  978
activity_report_summary.xml  =>  no match
activity_report_summary_justsometingwrong.xml  =>  no match
```

### SRCH-MAT-017 case 1: consecutive pattern digits cannot skip over a different name digit
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
12
123
13
1x2
```
keys: match "12"
```after
[12]  =>  1005
[12]3  =>  1004
13  =>  no match
[1]x[2]  =>  994
```

### SRCH-MAT-017 case 2: digits are word starts
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
a1b2
a13
a12
```
keys: match "a12"
```after
[a1]b[2]  =>  995
a13  =>  no match
[a12]  =>  1006
```

### SRCH-MAT-017 case 3: a lone digit after a star
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
a2b
2b
ab
```
keys: match "*2"
```after
a[2]b  =>  2
[2]b  =>  1003
ab  =>  no match
```

### SRCH-MAT-018 case 1: a short middle fragment is fine when it ends the pattern
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
Map
hashmap
```
keys: match "*ap"
```after
M[ap]  =>  2
hashm[ap]  =>  2
```

### SRCH-MAT-018 case 2: a two-letter middle fragment followed by more pattern fails
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
hashmap
hashMap
```
keys: match "*am"
```after
hashmap  =>  no match
hashMap  =>  no match
```

### SRCH-MAT-018 case 3: three letters are enough
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
MapEbc
MapEx
```
keys: match "*apeb"
```after
M[apEb]c  =>  0
MapEx  =>  no match
```

### SRCH-MAT-018 case 4: two are not
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
MapEbc
```
keys: match "*aeb"
```after
MapEbc  =>  no match
```

### SRCH-MAT-019 case 1: a prefix fragment is re-anchored on uppercase letters
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
CurrentUser
CuteUser
CUser
```
keys: match "CU"
```after
[C]urrent[U]ser  =>  1101
[C]ute[U]ser  =>  1101
[CU]ser  =>  1102
```

### SRCH-MAT-019 case 2: an acronym keeps its capitals
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
URLConnection
UrlUtil
UserResourceLocator
```
keys: match "URL"
```after
[URL]Connection  =>  1152
[Url]Util  =>  1052
[U]ser[R]esource[L]ocator  =>  1150
```

### SRCH-MAT-019 case 3: lowercase letters skip to humps
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
getUserName
GetUserName
guName
```
keys: match "gun"
```after
[g]et[U]ser[N]ame  =>  981
[G]et[U]ser[N]ame  =>  979
[guN]ame  =>  1002
```

### SRCH-MAT-020 case 1: a trailing space needs a space after the last fragment
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
foo bar
foo
foobar
```
keys: match "foo "
```after
[foo ]bar  =>  1004
[foo]  =>  1004
foobar  =>  no match
```

### SRCH-MAT-020 case 2: no requirement after an uppercase last character
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
foo bar
fooBar
foobar
```
keys: match "Foo "
```after
[foo ]bar  =>  1002
fooBar  =>  no match
foobar  =>  no match
```

### SRCH-MAT-030 case 1: a whole-word start earns 1000
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
Map
HashMap
hashmap
MapHash
```
keys: match "*map"
```after
[Map]  =>  1002
Hash[Map]  =>  1001
hash[map]  =>  2
[Map]Hash  =>  1001
```

### SRCH-MAT-031 case 1: fewer fragments rank higher
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
PROVIDER
ProjectView
ProViewer
```
keys: match "ProVi"
```after
[PROVI]DER  =>  1099
[Pro]ject[Vi]ew  =>  1101
[ProVi]ewer  =>  1102
```

### SRCH-MAT-031 case 2: every skipped hump between fragments costs 10
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
AaBb
AaXxBb
AaXxYyBb
```
keys: match "ab"
```after
[A]a[B]b  =>  990
[A]aXx[B]b  =>  980
[A]aXxYy[B]b  =>  970
```

### SRCH-MAT-032 case 1: start-match and end-match bonuses
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
file
fileIndex
e
e1
```
keys: match "*e"
```after
fil[e]  =>  2
fil[e]Index  =>  1
[e]  =>  1004
[e]1  =>  1003
```

### SRCH-MAT-033 case 1: uppercase pattern letter on an uppercase hump is worth 50
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
Hashmap
hashmap
```
keys: match "*Map"
```after
Hash[Map]  =>  1052
Hash[map]  =>  2
hash[map]  =>  2
```

### SRCH-MAT-033 case 2: lowercase pattern letter on a capital hump is worth 1
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
HashMap
Hashmap
hashmap
```
keys: match "*map"
```after
Hash[Map]  =>  1001
Hash[map]  =>  2
hash[map]  =>  2
```

### SRCH-MAT-033 case 3: case of a hump start, lowercase pattern
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
Boolean
boolean
```
keys: match "boo"
```after
[Boo]lean  =>  1001
[boo]lean  =>  1003
```

### SRCH-MAT-033 case 4: case of a hump start, capitalised pattern
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
Boolean
boolean
```
keys: match "Boo"
```after
[Boo]lean  =>  1052
[boo]lean  =>  1001
```

### SRCH-MAT-033 case 5: camel humps beat an all-capitals name
settings: mode=IGNORE_CASE, prefer-start=off
language: plain
```before
MarkUp
MavenUmlProvider
MyUserProfile
```
keys: match "MUP"
```after
[M]ark[Up]  =>  1102
[M]aven[U]ml[P]rovider  =>  1150
[M]y[U]ser[P]rofile  =>  1150
```

### SRCH-MAT-034 case 1: typing map
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "map"
```after
Map  [Map]  11002
MapHash  [Map]Hash  11001
ConcurrentHashMap  ConcurrentHash[Map]  1001
HashMap  Hash[Map]  1001
LinkedHashMap  LinkedHash[Map]  1001
MyHashMap  MyHash[Map]  1001
HashMapTest  Hash[Map]Test  1000
```

### SRCH-MAT-034 case 2: typing *map
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "*map"
```after
Map  [Map]  1002
ConcurrentHashMap  ConcurrentHash[Map]  1001
HashMap  Hash[Map]  1001
LinkedHashMap  LinkedHash[Map]  1001
MapHash  [Map]Hash  1001
MyHashMap  MyHash[Map]  1001
HashMapTest  Hash[Map]Test  1000
```

### SRCH-MAT-040 case 1: typing hm
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "hm"
```after
HashMap  [H]ash[M]ap  10990
HashMapTest  [H]ash[M]apTest  10990
ConcurrentHashMap  Concurrent[H]ash[M]ap  989
LinkedHashMap  Linked[H]ash[M]ap  989
MyHashMap  My[H]ash[M]ap  989
```

### SRCH-MAT-040 case 2: typing HM
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "HM"
```after
HashMap  [H]ash[M]ap  11101
HashMapTest  [H]ash[M]apTest  11101
ConcurrentHashMap  Concurrent[H]ash[M]ap  1100
LinkedHashMap  Linked[H]ash[M]ap  1100
MyHashMap  My[H]ash[M]ap  1100
```

### SRCH-MAT-040 case 3: typing hash
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "hash"
```after
HashMap  [Hash]Map  11001
HashMapTest  [Hash]MapTest  11001
HashSet  [Hash]Set  11001
Hashtable  [Hash]table  11001
MapHash  Map[Hash]  1001
ConcurrentHashMap  Concurrent[Hash]Map  1000
LinkedHashMap  Linked[Hash]Map  1000
MyHashMap  My[Hash]Map  1000
```

### SRCH-MAT-040 case 4: typing Hash
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "Hash"
```after
HashMap  [Hash]Map  11052
HashMapTest  [Hash]MapTest  11052
HashSet  [Hash]Set  11052
Hashtable  [Hash]table  11052
MapHash  Map[Hash]  1052
ConcurrentHashMap  Concurrent[Hash]Map  1051
LinkedHashMap  Linked[Hash]Map  1051
MyHashMap  My[Hash]Map  1051
```

### SRCH-MAT-040 case 5: typing hamap
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "hamap"
```after
HashMap  [Ha]sh[Map]  10991
HashMapTest  [Ha]sh[Map]Test  10990
ConcurrentHashMap  Concurrent[Ha]sh[Map]  990
LinkedHashMap  Linked[Ha]sh[Map]  990
MyHashMap  My[Ha]sh[Map]  990
```

### SRCH-MAT-040 case 6: typing ap
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "ap"
```after
ConcurrentHashMap  ConcurrentHashM[ap]  2
HashMap  HashM[ap]  2
LinkedHashMap  LinkedHashM[ap]  2
Map  M[ap]  2
MyHashMap  MyHashM[ap]  2
HashMapTest  HashM[ap]Test  1
MapHash  M[ap]Hash  1
```

### SRCH-MAT-040 case 7: typing s
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "s"
```after
Set  [S]et  11001
SortedSet  [S]ortedSet  11001
```

### SRCH-MAT-040 case 8: typing Li
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "Li"
```after
LinkedHashMap  [Li]nkedHashMap  11052
LinkedList  [Li]nkedList  11052
List  [Li]st  11052
AbstractList  Abstract[Li]st  1051
ArrayList  Array[Li]st  1051
```

### SRCH-MAT-040 case 9: typing AL
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "AL"
```after
AbstractList  [A]bstract[L]ist  11101
ArrayList  [A]rray[L]ist  11101
ThreadLocal  ThreadLoc[al]  2
```

### SRCH-MAT-040 case 10: typing tl
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
HashMap
HashSet
Hashtable
LinkedHashMap
ConcurrentHashMap
HashMapTest
MapHash
Map
MyHashMap
Set
SortedSet
ArrayList
LinkedList
List
AbstractList
ThreadLocal
```
keys: Go to Class, type "tl"
```after
ThreadLocal  [T]hread[L]ocal  10990
AbstractList  Abstrac[tL]ist  0
```

### SRCH-MAT-040 case 11: typing sb
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "sb"
```after
StringBuffer  [S]tring[B]uffer  10990
StringBuilder  [S]tring[B]uilder  10990
```

### SRCH-MAT-040 case 12: typing SBu
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "SBu"
```after
StringBuffer  [S]tring[Bu]ffer  11101
StringBuilder  [S]tring[Bu]ilder  11101
```

### SRCH-MAT-040 case 13: typing str
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "str"
```after
String  [Str]ing  11001
StringBuffer  [Str]ingBuffer  11001
StringBuilder  [Str]ingBuilder  11001
StringJoiner  [Str]ingJoiner  11001
StringUtil  [Str]ingUtil  11001
```

### SRCH-MAT-040 case 14: typing url
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "url"
```after
URL  [URL]  11002
URLConnection  [URL]Connection  11001
UrlUtil  [Url]Util  11001
```

### SRCH-MAT-040 case 15: typing URL
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "URL"
```after
URL  [URL]  11153
URLConnection  [URL]Connection  11152
UrlUtil  [Url]Util  11052
```

### SRCH-MAT-040 case 16: typing xml
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "xml"
```after
XMLHttpRequest  [XML]HttpRequest  11001
XmlParser  [Xml]Parser  11001
```

### SRCH-MAT-040 case 17: typing tp
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "tp"
```after
ThreadPoolExecutor  [T]hread[P]oolExecutor  10990
XMLHttpRequest  XMLHt[tp]Request  1
```

### SRCH-MAT-040 case 18: typing TPE
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "TPE"
```after
ThreadPoolExecutor  [T]hread[P]ool[E]xecutor  11150
```

### SRCH-MAT-040 case 19: typing te
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "te"
```after
TimeoutException  [T]imeout[E]xception  10990
ThreadPoolExecutor  [T]hreadPool[E]xecutor  10980
SimpleDateFormat  SimpleDa[te]Format  1
```

### SRCH-MAT-040 case 20: typing thread
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
StringBuilder
StringBuffer
StringUtil
String
StringJoiner
SimpleDateFormat
URLConnection
UrlUtil
URL
XMLHttpRequest
XmlParser
Thread
ThreadPoolExecutor
ThreadLocal
TimeoutException
```
keys: Go to Class, type "thread"
```after
Thread  [Thread]  11002
ThreadLocal  [Thread]Local  11001
ThreadPoolExecutor  [Thread]PoolExecutor  11001
```

### SRCH-MAT-041 case 1: a name equal to the typed text ignoring case goes first
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
ArrayList
LinkedList
List
ListIterator
Listener
SubList
Lists
LISTENER
listing
```
keys: Go to Class, type "list"
```after
List  [List]  11002
listing  [list]ing  11003
Listener  [List]ener  11001
LISTENER  [LIST]ENER  11001
ListIterator  [List]Iterator  11001
Lists  [List]s  11001
ArrayList  Array[List]  1001
LinkedList  Linked[List]  1001
SubList  Sub[List]  1001
```

### SRCH-MAT-041 case 2: same text with a capital
settings: mode=IGNORE_CASE, middle-matching=on, prefer-start=on
language: plain
```before
ArrayList
LinkedList
List
ListIterator
Listener
SubList
Lists
LISTENER
listing
```
keys: Go to Class, type "List"
```after
List  [List]  11053
Listener  [List]ener  11052
ListIterator  [List]Iterator  11052
Lists  [List]s  11052
LISTENER  [LIST]ENER  11049
listing  [list]ing  11001
ArrayList  Array[List]  1052
LinkedList  Linked[List]  1052
SubList  Sub[List]  1052
```

### SRCH-MAT-044 case 1: unqualified
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "StringUtil"
```after
com.acme.text.StringUtil  11103
com.acme.util.StringUtil  11103
org.other.util.StringUtil  11103
com.acme.util.StringUtils  11102
com.acme.StringUtilTest  11102
```

### SRCH-MAT-035 case 1: qualifier narrows and the exact match gets 5000
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "util.StringUtil"
```after
com.acme.util.StringUtil  16103  (exact qualified match, +5000)
org.other.util.StringUtil  16103  (exact qualified match, +5000)
com.acme.util.StringUtils  11102
```

### SRCH-MAT-035 case 2: two qualifier segments
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "acme.util.StringUtil"
```after
com.acme.util.StringUtil  16103  (exact qualified match, +5000)
com.acme.util.StringUtils  11102
```

### SRCH-MAT-035 case 3: exact suffix beats a prefix match of a longer name
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "model.User"
```after
com.acme.model.User  16053  (exact qualified match, +5000)
net.x.model.User  16053  (exact qualified match, +5000)
com.acme.model.UserService  11052
```

### SRCH-MAT-044 case 2: a qualifier that is not adjacent is only a filter
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "acme.User"
```after
com.acme.model.User  11053
com.acme.web.UserController  11052
com.acme.model.UserService  11052
```

### SRCH-MAT-044 case 3: abbreviated qualifier segments
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "a.m.U"
```after
com.acme.model.User  11052
com.acme.model.UserService  11052
```

### SRCH-MAT-035 case 4: one candidate left
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "text.StringUtil"
```after
com.acme.text.StringUtil  16103  (exact qualified match, +5000)
```

### SRCH-MAT-035 case 5: package-like qualifier
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.util.StringUtils
com.acme.text.StringUtil
com.acme.StringUtilTest
com.acme.util.Strings
net.x.model.User
com.acme.model.User
com.acme.model.UserService
com.acme.web.UserController
org.other.User
```
keys: type "other.User"
```after
org.other.User  16053  (exact qualified match, +5000)
```

### SRCH-MAT-042 case 1: splitting the typed text into name and position
settings: Go to File or Go to Class
language: plain
```before
Foo
Foo:12
Foo:12:5
Foo:12,5
Foo 12
Foo(12)
Foo(12:5)
Foo[12]
Foo#L12
Foo?l=12
Foo on line 12
Foo at line 12
Foo:line 12
Foo@12
Foo,12
Foo::5
Foo:
Foo:abc
Foo.java:7
src/Foo.java:7
Foo 
Foo:12 
Foo:0
Foo:12:0
My File.txt:3
Foo:12:5:7
```
keys: typing each text in turn
```after
'Foo'                  -> name='Foo', line=-, column=-
'Foo:12'               -> name='Foo', line=12, column=1
'Foo:12:5'             -> name='Foo', line=12, column=5
'Foo:12,5'             -> name='Foo', line=12, column=5
'Foo 12'               -> name='Foo', line=12, column=1
'Foo(12)'              -> name='Foo', line=12, column=1
'Foo(12:5)'            -> name='Foo', line=12, column=5
'Foo[12]'              -> name='Foo', line=12, column=1
'Foo#L12'              -> name='Foo', line=12, column=1
'Foo?l=12'             -> name='Foo', line=12, column=1
'Foo on line 12'       -> name='Foo', line=12, column=1
'Foo at line 12'       -> name='Foo', line=12, column=1
'Foo:line 12'          -> name='Foo', line=12, column=1
'Foo@12'               -> name='Foo', line=12, column=1
'Foo,12'               -> name='Foo', line=12, column=1
'Foo::5'               -> name='Foo', line=1, column=5
'Foo:'                 -> name='Foo', line=-, column=-
'Foo:abc'              -> name='Foo:abc', line=-, column=-
'Foo.java:7'           -> name='Foo.java', line=7, column=1
'src/Foo.java:7'       -> name='src/Foo.java', line=7, column=1
'Foo '                 -> name='Foo ', line=-, column=-
'Foo:12 '              -> name='Foo ', line=12, column=1
'Foo:0'                -> name='Foo', line=-, column=-
'Foo:12:0'             -> name='Foo', line=12, column=1
'My File.txt:3'        -> name='My File.txt', line=3, column=1
'Foo:12:5:7'           -> name='Foo:12', line=5, column=7
```

### SRCH-MAT-046 case 1: exact-case prefix tier
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "README"
```after
README.md  11302
```

### SRCH-MAT-046 case 2: lowercase typing finds the same file
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "readme"
```after
README.md  11001
```

### SRCH-MAT-046 case 3: exact-case prefix tier first, then by degree, then name without extension
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "us"
```after
docs/user-guide.md  11003
src/main/resources/user-service.properties  11003
src/main/java/com/acme/UserController.java  11001
src/main/java/com/acme/UserService.java  11001
src/main/java/com/other/UserService.java  11001
src/test/java/com/acme/UserServiceTest.java  11001
```

### SRCH-MAT-046 case 4: capitals pick the camel-hump tier
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "US"
```after
src/main/java/com/acme/UserService.java  11101
src/main/java/com/other/UserService.java  11101
src/test/java/com/acme/UserServiceTest.java  11101
src/main/java/com/acme/UserController.java  11052
docs/user-guide.md  11001
src/main/resources/user-service.properties  10989
```

### SRCH-MAT-046 case 5: extension in the name part
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "UserService.java"
```after
src/main/java/com/acme/UserService.java  11105
src/main/java/com/other/UserService.java  11105
src/test/java/com/acme/UserServiceTest.java  11094
```

### SRCH-MAT-046 case 6: middle match
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "gradle"
```after
build.gradle  1003
```

### SRCH-MAT-046 case 7: star-decorated pattern: word starts after hyphen or hump
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "service"
```after
src/main/resources/user-service.properties  1002
src/main/java/com/acme/UserService.java  1000
src/main/java/com/other/UserService.java  1000
src/test/java/com/acme/UserServiceTest.java  1000
```

### SRCH-MAT-045 case 1: directory part: shallower parent paths first
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "src/main/Us"
```after
src/main/resources/user-service.properties  996
src/main/java/com/acme/UserController.java  992
src/main/java/com/acme/UserService.java  992
src/main/java/com/other/UserService.java  991
```

### SRCH-MAT-045 case 2: directory part without the first segment
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "main/Us"
```after
src/main/resources/user-service.properties  993
src/main/java/com/acme/UserController.java  989
src/main/java/com/acme/UserService.java  989
src/main/java/com/other/UserService.java  988
```

### SRCH-MAT-045 case 3: one directory match
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "test/Us"
```after
src/test/java/com/acme/UserServiceTest.java  989
```

### SRCH-MAT-045 case 4: directory segment in the middle of the path
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "acme/Us"
```after
src/main/java/com/acme/UserController.java  1004
src/main/java/com/acme/UserService.java  1004
src/test/java/com/acme/UserServiceTest.java  1004
```

### SRCH-MAT-042 case 2: position suffix does not change the file list
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "UserService:12"
```after
src/main/java/com/acme/UserService.java  11102
src/main/java/com/other/UserService.java  11102
src/test/java/com/acme/UserServiceTest.java  11102
src/main/resources/user-service.properties  10989
```

### SRCH-MAT-042 case 3: position suffix does not change the file list
settings: Go to File, scope=Project, current file=none
language: plain
```before
src/main/java/com/acme/UserService.java
src/main/java/com/acme/UserController.java
src/test/java/com/acme/UserServiceTest.java
src/main/resources/application.yml
src/main/resources/user-service.properties
docs/user-guide.md
README.md
build.gradle
src/main/java/com/acme/util/StringUtil.java
src/main/java/com/other/UserService.java
```
keys: type "UserService.java:12:5"
```after
src/main/java/com/acme/UserService.java  11105
src/main/java/com/other/UserService.java  11105
src/test/java/com/acme/UserServiceTest.java  11094
```

### SRCH-MAT-043 case 1: class and member in one pattern (Go to Class)
settings: Go to Class, language=java
language: java
```before
--- p/Outer.java
package p;
public class Outer { void run() {} void walk() {} }
```
keys: type "Outer#ru"
```after
list: Outer
on Enter: Outer.java opens with the caret on `run` (member part `ru` matched as a prefix of members; the list itself shows only classes)
```

### SRCH-MAT-043 case 2: anonymous class path
settings: Go to Class, language=java
language: java
```before
--- p/Outer.java
package p;
public class Outer { Object a = new Object() {}; Object b = new Object() {}; }
```
keys: type "Outer$2", Enter
```after
Outer.java opens with the caret at the second anonymous class (`new Object() {}` of field b)
```

### SRCH-MAT-043 case 3: symbol with signature
settings: Go to Symbol, language=java
language: java
```before
--- p/Outer.java
package p;
public class Outer { void run(int x) {} }
```
keys: type "Outer#run(int)"
```after
list: Outer.run(int)   (the signature text in the parentheses is dropped for matching, `Outer#run` is matched against the qualified member name)
```

### SRCH-MAT-047 case 1: star inside the pattern
settings: Go to Class, separators=".", middle-matching=on
language: plain
```before
com.acme.util.StringUtil
org.other.util.StringUtil
com.acme.text.Strings
```
keys: type "acme*Util"
```after
com.acme.util.StringUtil
```
