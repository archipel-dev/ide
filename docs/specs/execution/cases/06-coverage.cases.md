# Cases: coverage (EXE-COV)

### EXE-COV-002 case 1: prompt after a coverage run
settings: Coverage option "When New Coverage Is Gathered" = Show options before applying coverage to the editor
language: java
```before
Application: Main finished a run with coverage
```
keys: wait for the end of the run
```after
dialog text: Do you want to display coverage data for 'Main Coverage Results'?
buttons: Replace Active Suites, Add to Active Suites, Do Not Apply Collected Coverage
```

### EXE-COV-006 case 1: chooser with no suites
settings: default
language: plain
```before
no coverage data
```
keys: Ctrl+Alt+F6
```after
popup title: Choose Coverage Suite to Display
text: No coverage suites configured.
```
