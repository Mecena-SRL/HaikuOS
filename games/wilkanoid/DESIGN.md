# Wilkanoid v2: progetto

Pilastri:
1. Fisica leggibile: una palla, un modello di rimbalzo (cerchio contro rettangolo), velocità che cresce per livello.
2. Un meccanismo nuovo per livello: normali, acciaio (due colpi), esplosivi (rompono i vicini). Ogni livello ne introduce o combina uno.
3. Feedback: lampo sul colpo, particelle, scossa breve. Niente audio (nessun driver in `no_std`).
4. Power-up pochi e chiari: racchetta larga, multipalla.

Non è un clone: gli esplosivi cambiano il modo di pianificare i colpi.

Stato: sei livelli, il bot li completa. Restano da fare: livello di tutorial, pausa, input su framebuffer.
