Nordic provides reference designs for Altium only. This is the nRF52832-QFAA_Ref_Layout_v0.9.zip reference layout

Ideally you could use altium's free www.circuitmaker.com to view the paid Circuit Designer files, but thats not the case unless they have already previously been exported as an autocad file:
https://www.youtube.com/watch?v=YAWBsShCId4

As thats not the case for these Nordic files I decided to play.

In the end I finally found a 'free' viewer (for 6 months) from Altium that is really all you need to get a look at the design decisions.
http://www.altium.com/altium-designer-viewer 

However I wanted to play with kicad and https://github.com/thesourcerer8/altium2kicad to see how close it gets

Fixed issues:
* Schematic wouldnt come over for some reason, I had to build that by hand.
* The Polygons didn't come in quite right, I made them Thermal Releif's and Polygons and that seemed to help.
* Keepout Polys were lost and recreated by hand
* Stiching Vias are.. problematic, in kicad so those needed to be handled by hand https://forum.kicad.info/t/protip-nicer-via-stitching/1103/32
* File-Archive Footprints->Create library Exported the footprints out of the design into a proper library, and moved the .3dshapes locally as well

Known issues:
* Altium has the no poly islands smaller than xsq but kicad doesnt appear to which means a few little islands exist on our gerbers that dont on theirs.