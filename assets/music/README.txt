Put .ogg files (Vorbis, 44.1 kHz stereo) into these folders. Any file name works.

graveyard/        map 1 background songs (shuffled)
graveyard/boss/   boss songs on map 1 (lords without their own songs)
cathedral/        map 2 background songs
cathedral/boss/   boss songs on map 2
flesh/            map 3 background songs
flesh/boss/       boss songs on map 3
ashwood/          map 4 (The Ashwood); empty: uses graveyard/
ashwood/boss/     boss songs on map 4
abbey/            map 5 (The Frozen Abbey); empty: uses cathedral/
abbey/boss/       boss songs on map 5
garden/           map 6 (The Starless Garden); empty: uses flesh/
garden/boss/      boss songs on map 6
boss/             boss songs for any map whose own boss folder is empty
boss/<lord>/      a lord's own songs, used first wherever it appears:
                    gravewarden  The Gravewarden      (maps 1, 4)
                    hound        The Pale Hound       (maps 1, 3, 4, 6)
                    matriarch    Pale Matriarch       (maps 1, 3, 4, 6)
                    bogmother    The Bog Mother       (maps 1, 3, 6)
                    hermit       The Tolling Hermit   (maps 1, 2, 5)
                    seraph       The Stone Seraph     (maps 2, 5)
                    choir        The Hollow Choir     (maps 2, 5)
                    penitent     The Iron Penitent    (maps 2, 3, 4)
                    bride        The Drowned Bride    (maps 2, 3, 5, 6)
                    burntoak     The Burnt Oak        (map 4)
                    snowabbess   The Snow Abbess      (map 5)
                    gardener     The Gardener         (map 6)
                    sovereign    Ashen Sovereign      (the last hour, every map)
                  A lord with an empty folder uses <map>/boss/, then boss/.
menu/             main menu (falls back to graveyard)
endless/          endless mode (falls back to the map's songs)
rush/             boss rush
lullaby/          lullaby moments
secret/           the main menu easter egg (one song picked at random)

An empty folder keeps the built-in synthesized music for that part.
Rebuild the game (cargo build) after adding songs so they are embedded in the exe.
