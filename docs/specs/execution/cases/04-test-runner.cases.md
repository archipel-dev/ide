# Cases: test runner (EXE-TEST)

Event streams are listed as the lines the test process prints (service messages shortened; `|n`
escapes are written out). Tree blocks show `<indent><icon-state> name <duration>`.

### EXE-TEST-001 case 1: aggregation
settings: default
language: plain
```before
stream:
##teamcity[testingStarted]
##teamcity[testSuiteStarted name='FooTest']
##teamcity[testStarted name='a']
##teamcity[testFinished name='a' duration='5']
##teamcity[testStarted name='b']
##teamcity[testFailed name='b' message='boom']
##teamcity[testFinished name='b' duration='7']
##teamcity[testStarted name='c']
##teamcity[testFinished name='c' duration='1']
##teamcity[testSuiteFinished name='FooTest']
##teamcity[testingFinished]
```
keys: view the tree (Show Passed on)
```after
Test Results                      [failed]  13 ms
  FooTest                         [failed]  13 ms
    a                             [passed]  5 ms
    b                             [failed]  7 ms
    c                             [passed]  1 ms
status line: Done: 3 of 3  Failed: 1  (13 ms)
```

### EXE-TEST-003 case 1: counters with ignore and not started
settings: default
language: plain
```before
stream:
##teamcity[enteredTheMatrix]
##teamcity[testCount count='5']
##teamcity[testStarted name='a']  ##teamcity[testFinished name='a' duration='1']
##teamcity[testStarted name='b']  ##teamcity[testIgnored name='b' message='later']  ##teamcity[testFinished name='b' duration='0']
##teamcity[testStarted name='c']  ##teamcity[testFailed name='c' message='x']  ##teamcity[testFinished name='c' duration='2']
process ends (testingFinished printed)
```
keys: read the notification
```after
counters: total 5, passed 1, ignored 1, failed 1, not started 2
balloon: Tests failed: 1, passed: 1, ignored: 1, not started: 2
```

### EXE-TEST-010 case 1: late failure and unknown finish
settings: default
language: plain
```before
stream:
##teamcity[testStarted name='a']
##teamcity[testFinished name='a' duration='3']
##teamcity[testFailed name='a' message='late']
##teamcity[testFinished name='ghost' duration='1']
```
keys: view the tree
```after
a   [failed]  3 ms
(no node for ghost; no exception)
```

### EXE-TEST-012 case 1: process dies mid-test
settings: default
language: plain
```before
stream:
##teamcity[enteredTheMatrix]
##teamcity[testStarted name='a']
process exits with code 137 before testFinished
```
keys: view the tree
```after
root text: Test Results (terminated state)
a   [terminated]
console ends with: Process finished with exit code 137 (interrupted by signal 9:SIGKILL)
```

### EXE-TEST-012 case 2: no handshake
settings: default
language: plain
```before
process prints "hello" and exits 0, no service messages
```
keys: view the tree
```after
root: Test framework quit unexpectedly
```

### EXE-TEST-012 case 3: handshake but no tests
settings: default
language: plain
```before
stream: ##teamcity[enteredTheMatrix] then exit
```
keys: view the tree
```after
root: No tests were found
```

### EXE-TEST-021 case 1: root labels
settings: default
language: plain
```before
(a) just started, no events; (b) running with children; (c) all passed; (d) all finished, one ignored, rest passed
```
keys: read the root label in each state
```after
(a) Instantiating tests...
(b) Running tests...
(c) All Tests Passed
(d) All Tests Passed (except ignored)
```

### EXE-TEST-025 case 1: Show Passed off
settings: default
language: plain
```before
tree: FooTest [failed] { a [passed], b [failed], c [ignored], d [passed] }
```
keys: toggle Show Passed off
```after
FooTest [failed]
  b [failed]
  c [ignored]
counters unchanged: total 4
```

### EXE-TEST-040 case 1: comparison failure
settings: default
language: java
```before
stream:
##teamcity[testStarted name='eq']
##teamcity[testFailed name='eq' message='expected: <a> but was: <b>' details='java.lang.AssertionError|n	at p.FooTest.eq(FooTest.java:9)|n' type='comparisonFailure' expected='a' actual='b']
##teamcity[testFinished name='eq' duration='4']
```
keys: select `eq`; click `<Click to see difference>`
```after
output pane lines:
expected: <a> but was: <b>
Expected :a
Actual   :b
<Click to see difference>
java.lang.AssertionError
	at p.FooTest.eq(FooTest.java:9)
diff window: left title Expected, text a; right title Actual, text b
```

### EXE-TEST-041 case 1: no comparison data
settings: default
language: plain
```before
tree with a passed test selected
```
keys: action View Assert Equals Diff
```after
dialog title: No Comparison Data Found
dialog text: Comparison error was not found
```

### EXE-TEST-050 case 1: rerun failed tests
settings: default
language: java
```before
JUnit: FooTest run finished: a [passed], b [failed], c [failed]
```
keys: click Rerun Failed Tests
```after
a new run starts of the same configuration restricted to b and c
tree after the run: only b and c (a not present)
```

### EXE-TEST-053 case 1: all passed balloon
settings: default
language: plain
```before
run with 7 passed tests; Run window hidden
```
keys: wait for the end
```after
balloon (title Tests Passed): Tests passed: 7
```

### EXE-TEST-053 case 2: ignored only
settings: default
language: plain
```before
run with 3 passed, 2 ignored
```
keys: wait for the end
```after
balloon (title Tests Ignored): Tests ignored: 2, passed: 3
```

### EXE-TEST-053 case 3: failure and interruption
settings: default
language: plain
```before
(a) 1 failed, 4 passed; (b) the user pressed Stop after starting
```
keys: read both balloons
```after
(a) title Tests Failed: Tests failed: 1, passed: 4
(b) Tests were interrupted
```
