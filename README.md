# EelFile - for all your insecure* file transfer needs

Want to transfer a file to a friend and Discord has a file limit that's too high? Using Google Disk or one of those file sharing services seems like too easy of a solution?

DOWNLOAD EELFILE, WRITTEN 100% IN RUST

*You know it's not a virus because it's open source :^)*

![image](https://github.com/user-attachments/assets/8ad22a75-a0b7-405b-a852-45a2d8e089f7)

Requirements:

Windows (tested on 10 and 11)
Forwarded ports if you want to receive files (you didn't think it'd be so easy?)

Bonus feature: random eel facts

Known issues:

- file size won't display or log properly if the file is over 1TiB or larger
- the free space check won't trigger correctly if you manually type the relative path to your file (but why would you)
- animation transitions are sometimes borked and the status gif doesn't start at the start. Unfortunately without implementing my own animation system instead of using eframe's I'm not sure I can fix it
- the app is impractical and useless (will not be fixed)

\* Encryption support planned eventually soon™
