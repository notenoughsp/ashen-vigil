Put .ogg files (Vorbis, 44.1 kHz stereo) into these folders. Any file name works.

graveyard/        map 1 background songs (shuffled)
graveyard/boss/   boss songs on map 1 (lords without their own songs)
cathedral/        map 2 background songs
cathedral/boss/   boss songs on map 2
flesh/            map 3 background songs
flesh/boss/       boss songs on map 3
boss/             boss songs for any map whose own boss folder is empty
boss/<lord>/      a lord's own songs, used first wherever it appears:
                    gravewarden  The Gravewarden      (map 1)
                    hound        The Pale Hound       (maps 1, 3)
                    matriarch    Pale Matriarch       (maps 1, 3)
                    bogmother    The Bog Mother       (maps 1, 3)
                    hermit       The Tolling Hermit   (maps 1, 2)
                    seraph       The Stone Seraph     (map 2)
                    choir        The Hollow Choir     (map 2)
                    penitent     The Iron Penitent    (maps 2, 3)
                    bride        The Drowned Bride    (maps 2, 3)
                    sovereign    Ashen Sovereign      (the last hour, every map)
                  A lord with an empty folder uses <map>/boss/, then boss/.
menu/             main menu (falls back to graveyard)
endless/          endless mode (falls back to the map's songs)
rush/             boss rush
lullaby/          lullaby moments
secret/           the main menu easter egg (one song picked at random)

An empty folder keeps the built-in synthesized music for that part.
Rebuild the game (cargo build) after adding songs so they are embedded in the exe.
