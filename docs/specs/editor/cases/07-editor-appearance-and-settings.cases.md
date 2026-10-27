# Cases for 07 appearance and scrolling

All viewport cases are `status: unverified (scroll mechanics derived from the settings texts)`.

### ED-VIEW-020 case 1: moving down into the scroll margin scrolls one line
settings: default
status: unverified (scroll mechanics derived from the settings texts)
language: plain
viewport: 5 lines, top=0
```before
0
1
2
3
<caret>4
5
6
7
```
keys: Down
```after
0
1
2
3
4
<caret>5
6
7
```
after viewport: top=1

### ED-VIEW-020 case 2: moving inside the margin does not scroll
settings: default
status: unverified (scroll mechanics derived from the settings texts)
language: plain
viewport: 5 lines, top=0
```before
0
1
<caret>2
3
4
5
```
keys: Down
```after
0
1
2
<caret>3
4
5
```
after viewport: top=0

### ED-VIEW-020 case 3: moving up into the top margin scrolls one line
settings: default
status: unverified (scroll mechanics derived from the settings texts)
language: plain
viewport: 5 lines, top=3
```before
0
1
2
3
<caret>4
5
6
7
8
```
keys: Up
```after
0
1
2
<caret>3
4
5
6
7
8
```
after viewport: top=2

### ED-VIEW-020 case 4: scroll offset 0
settings: Vertical scroll offset=0
status: unverified (scroll mechanics derived from the settings texts)
language: plain
viewport: 5 lines, top=0
```before
0
1
2
<caret>3
4
5
```
keys: Down, Down
```after
0
1
2
3
4
<caret>5
```
after viewport: top=1

### ED-VIEW-071 case 1: global font size increase
settings: editor font size=12
language: plain
```before
<caret>x
```
keys: Alt+Shift+Period
```after
<caret>x
```
after font size: 13

### ED-VIEW-071 case 2: the maximum size is not exceeded
settings: editor font size=40
language: plain
```before
<caret>x
```
keys: Alt+Shift+Period
```after
<caret>x
```
after font size: 40

### ED-VIEW-071 case 3: the minimum zoom-out size
settings: editor font size=8
language: plain
```before
<caret>x
```
keys: Alt+Shift+Comma
```after
<caret>x
```
after font size: 8
