# Introduction

This is a standalone application that runs alongside DCS (Digital Combat Simulator) to overlay important flight information (ie. speed, altitude, heading) on top of the game window, similar to real-life [Helmet-mounted displays](https://en.wikipedia.org/wiki/Helmet-mounted_display) such as the JHCMS in the F/A-18 and DASS in the Eurofighter Typhoon.

Demonstration: https://www.youtube.com/watch?v=HJA8nITgAQY

# User Guide

## Installation

* Download the latest release from https://github.com/ricmzn/dcs-hemmecs/releases/latest

* Enter the `%userprofile%\Saved Games\DCS` or `%userprofile%\Saved Games\DCS.openbeta` folder

* (First install only) Edit the `Scripts\Export.lua` file to insert the following two lines at the end:  
`local hemmecs, hemmecsErr = pcall(function() local hemmecsLfs=require('lfs'); dofile(hemmecsLfs.writedir()..'Scripts/HemmecsExport.lua'); end);`  
`if not hemmecs then log.write("HEMMECS.EXPORT", log.ERROR, hemmecsErr) end;`

* Extract the `Scripts\HemmecsExport.lua` file from the release into the `Scripts` folder

* Extract `dcs-hemmecs.exe` to any location

## Usage

* Load into a mission in DCS

* Run `dcs-hemmecs.exe`

* To close: either focus the application (click it in the taskbar or alt-tab into it) and press esc, or close it in task manager

# FAQ

Q: Does this pass IC?  
A: Yes, as it does not modify any game files directly, and uses the built-in export functionality, you can use it in multiplayer. However, some servers may disable the export functionality, causing the application to not work properly.

Q: All the numbers are zero!  
A: This means the application is not yet receiving data from DCS, either because there is no mission running, or the exporter script has encountered an error. Currently, a few errors are logged to the DCS.log file, but there is still a lot of work left in making it more stable.

Q: Can I change the units to metric?  
A: Not yet, unless you modify the source code and remove the conversions.

Q: How do I make it brighter/darker/bigger/smaller?  
A: No customizations are available yet.

Q: Can I use this in VR?  
A: Not yet, sorry. It would need to ouput the image to a compositor layer such as SteamVR instead of a desktop window.

# Known issues

* Versions 0.1.0 and 0.2.0 break TacView exports (sorry!)

* The application may hang in the background after closing it

* The code is bad and it doesn't deal with any edge cases, occupied ports, multi-monitor support, etc
