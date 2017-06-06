p!/usr/bin/env python3
import curses
import curses.ascii
import os

window = curses.initscr()
curses.noecho()
curses.cbreak()
window.keypad(True)
index = 0
msg = []

def list_current_dir():
    window.erase()

    content = os.listdir();
    content.insert(0,"..")
    attr = curses.A_NORMAL
    for i, entry in enumerate(content) :
        if i == index :
            attr = curses.A_REVERSE
        else:
            attr = curses.A_NORMAL

        if os.path.isdir(entry) :
            window.addstr(entry + "/\n" , attr)
        else:
            window.addstr(entry + "\n" , attr)

    window.addstr("\n")
    for entry in msg:
        window.addstr(entry + "\n", curses.A_REVERSE)
    msg.clear()

    window.refresh()

# main
home = os.path.expanduser("~/diary")
os.chdir(home)
list_current_dir()

while True:
    ch = window.getch()
    content = os.listdir()
    content.insert(0,"..")
    if (ch == curses.KEY_DOWN or ch == ord('j')) and index < len(os.listdir()) :
        index += 1
        list_current_dir()
    elif (ch == curses.KEY_UP or ch == ord('k')) and index != 0 :
        index -= 1
        list_current_dir()
    elif ch == curses.KEY_ENTER or ch == curses.ascii.LF or ch == curses.ascii.CR :
        if os.path.isdir(content[index]):
            os.chdir(content[index])
            index = 0
        else:
            msg.append("Opening File is not implemented yet")
        list_current_dir()
    elif ch == ord('q') or ch == ord('Q'):
        break;

curses.nocbreak()
curses.echo()
curses.endwin()
