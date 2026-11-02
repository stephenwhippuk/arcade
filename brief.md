Arcade is a project for practce game creation. Its purpose is to build an engine into which various old games can be reimagined and implemented within a consistent framework.

as this ibroadly educational, the project will be written in rust. 

core components
1) a shack based game-state that allows for laying of interface types. main meu, game, bring the menu up in game etc etc. This gives flexibility of interface both graphical and HCI without hard coding order of screens etc 
2) a simple 2D display (for now) based upon a suitable library for rust binding, e.g. (open GL) it should provide the primitives we need to construct a variaty of games
3) collision physics etc 
4) scripting Npn player entities. 
5) for now keyboard control but we should code to allow different modes of control 
6) sound, effects plus music and controlling of settings available for menu screens etc 