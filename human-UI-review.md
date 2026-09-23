# human UI/UX review of screenshots

## overall

- every layout feels hacked together can custom, when it should use basic tools to build from.
- padding is still bad everywhere
- I would like to see a more fixed modular design that enforces these issues
  - top bar is inconsistent -> make the top bar unchangable by individual pages
  - breadcrumbs level moves -> make the breadcrumbs location unchangable by individual pages
- overall the interface is too monotone, everything is bland
- yes a modular row display system would solve so many problems, we have so many row display pages and they are all custom
- the row display could support multiple views like reddit for instance. Does SolidJS have something like this? It would be better to import a library than build from scratch, but I am not opposed to building from scratch.

## instructor interface

- depsite my insistence on a locked down top ribbon UI and the exact ribbon UI specified in the human guidance, we are still changing the top ribbon UI: "Courses | Questions | Assessments" never changes. The top ribbon has to be the same for all pages an instructor will visit.
- the second level of the ribbon is allowed to change.
- the 'not available yet' makes the interface worse that a broken link
- starred and my questions need to be available, they are mostly just queries.
- for every display row of content, we have to ask ourselves, what information does the user care about. the assessment ID (probably not), which fixed time zone I am in for every date (never)
- row should feel more like spreadsheet rows, you should be able to reorder things by drag/drop or arrow keys, all columns are aligned.
  - perhaps a dedicated row display module that we can feed data too

### my draft questions
- why is the title listed twice
- why is private draft displayed, shouldn't it be obvious based on the page
- why does edit number take up five long words.
- maybe have a preview of the content

## student interface

- I need to see more than one webwork example
- overall the interface is too monotone, everything is bland, when taking an assignment the assigment should be the focus
  - the webwork backend gets like a white background, we should explore this for native json questions
- we should randomly assign the theme for the screenshots to get some variability for testing
- the top ribbon UI is a horrible mess and changes every single screenshot, top ribbon UI MUST BE FIXED and not change. THIS IS A REQUIREMENT of the interface, see About Face book. I have not quite decided what the top ribbon should be, but it has to be the same for all pages a student will visit.
- all student views should have the same number of screen captures. it is completely uneven
  - `ls docs/screenshots/student/tablet/*.png | wc -l` == 3
  - `ls docs/screenshots/student/phone/*.png | wc -l` == 22
  - `ls docs/screenshots/student/square/*.png | wc -l` == 1
  - `ls docs/screenshots/student/laptop/*.png | wc -l` == 30


### the student question taking on the phone

- question tracking
  - needs ellipses (...) between non-consecutive numbers
  - make the buttons more narrow to fit more numbers
- vertical space is wasted. screenshot is 852px tall and question does not start until 444px; 52% of the screenshot is heading when taking questions, ouch.
- I feel like the user role tag should always touch the logo in the upper right.
- for mobile mode, drop the "Peptidyle" work and just have the "P" logo
- why does the top ribbon change, all options all the time, Top level ribbon should be fixed for all user roles. It is the touchstone of the interface and provides user trust.

### in tablet mode

- in two screenshots we have a back page between the logo and breadcrumbs; and the other one the breadcrumbs are below the logo
- the logo / top ribbon UI is overly padded
- the breadcrumbs move position, this is bad.

### laptop mode
- restore response or undo is an overengineered feature; there 5 choices, WTF do you need an undo. Remove all stupid undo/restore functionality. no one asked for it and no one needs it.
- the webwork backend is over designed. We get (1) a horizontal rule, (2) black bordered window with (3) white background (4) medium gray bordered subwindow with (5) light gray background and then the question. Why so many layers?
- padding is a major issue
- questions start at 374px from top and first assessment starts 273px from the top.
- assessment selection page dedicates 326px for each assessment. I would make it more like 100-150px
- question mode has breadcrumbs in normal bar, but the top ribbon also becomes breadcrumbs. this is bad design.
- ordering up/down buttons should have arrows with the text
- ordering should support drag and drop as well as keyboard

## what is good

- consistent font
- consistent buttons
- objects that are clickable are generally obvious
