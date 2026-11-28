//! Città-impero: logica di gioco pura. Nessun I/O e nessun rendering;
//! il rendering vive in main.rs e legge solo lo stato pubblico.

pub const WIN_POP: u32 = 50;
const HOUSE_CAP: u32 = 5;
const IRRIGATION_COST: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Grass,
    Forest,
    Mountain,
    Water,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Building {
    House,
    Farm,
    Lumber,
    Quarry,
    Market,
    Library,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stock {
    pub food: i32,
    pub wood: i32,
    pub stone: i32,
    pub gold: i32,
}

impl Stock {
    pub const fn new(food: i32, wood: i32, stone: i32, gold: i32) -> Self {
        Stock {
            food,
            wood,
            stone,
            gold,
        }
    }

    fn covers(&self, c: &Stock) -> bool {
        self.food >= c.food && self.wood >= c.wood && self.stone >= c.stone && self.gold >= c.gold
    }

    /// Paga `c` solo se le risorse bastano: mai uno stato parziale.
    pub fn pay(&mut self, c: &Stock) -> bool {
        if !self.covers(c) {
            return false;
        }
        self.food -= c.food;
        self.wood -= c.wood;
        self.stone -= c.stone;
        self.gold -= c.gold;
        true
    }
}

impl Building {
    pub fn cost(self) -> Stock {
        match self {
            Building::House => Stock::new(0, 10, 0, 0),
            Building::Farm | Building::Lumber => Stock::new(0, 5, 0, 0),
            Building::Quarry => Stock::new(0, 8, 0, 0),
            Building::Market => Stock::new(0, 15, 5, 0),
            Building::Library => Stock::new(0, 10, 10, 0),
        }
    }

    /// Terreno obbligatorio; `None` vuol dire qualsiasi terreno non acquatico.
    pub fn terrain(self) -> Option<Terrain> {
        match self {
            Building::Farm => Some(Terrain::Grass),
            Building::Lumber => Some(Terrain::Forest),
            Building::Quarry => Some(Terrain::Mountain),
            _ => None,
        }
    }

    fn housing(self) -> u32 {
        if self == Building::House {
            HOUSE_CAP
        } else {
            0
        }
    }
}

#[derive(Clone, Debug)]
pub struct Map {
    pub w: usize,
    pub h: usize,
    terrain: Vec<Terrain>,
}

impl Map {
    /// Generatore deterministico (xorshift64): stesso seme, stessa mappa.
    pub fn generate(w: usize, h: usize, seed: u64) -> Self {
        let mut s = seed | 1;
        let terrain = (0..w * h)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                match s % 10 {
                    0..=3 => Terrain::Grass,
                    4..=5 => Terrain::Forest,
                    6..=7 => Terrain::Mountain,
                    _ => Terrain::Water,
                }
            })
            .collect();
        Map { w, h, terrain }
    }

    pub fn filled(w: usize, h: usize, t: Terrain) -> Self {
        Map {
            w,
            h,
            terrain: vec![t; w * h],
        }
    }

    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.w + x
    }

    pub fn terrain(&self, i: usize) -> Option<Terrain> {
        self.terrain.get(i).copied()
    }

    /// Rende erbosa un'area 3x3 attorno a (x, y): punto di partenza sicuro.
    fn clear_area(&mut self, x: usize, y: usize) {
        for yy in y.saturating_sub(1)..=(y + 1).min(self.h - 1) {
            for xx in x.saturating_sub(1)..=(x + 1).min(self.w - 1) {
                let i = self.idx(xx, yy);
                self.terrain[i] = Terrain::Grass;
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    OutOfBounds,
    Occupied,
    Unbuildable,
    WrongTerrain,
    NoResources,
}

/// Un impero: risorse, popolazione e le sue costruzioni sulla mappa condivisa.
#[derive(Clone, Debug)]
pub struct Empire {
    pub stock: Stock,
    pub knowledge: i32,
    pub citizens: u32,
    pub irrigation: bool,
    grid: Vec<Option<Building>>,
}

impl Empire {
    pub fn new(map: &Map, stock: Stock) -> Self {
        Empire {
            stock,
            knowledge: 0,
            citizens: 2,
            irrigation: false,
            grid: vec![None; map.w * map.h],
        }
    }

    pub fn building_at(&self, i: usize) -> Option<Building> {
        self.grid.get(i).copied().flatten()
    }

    pub fn count(&self, b: Building) -> usize {
        self.grid.iter().flatten().filter(|&&x| x == b).count()
    }

    pub fn capacity(&self) -> u32 {
        self.grid.iter().flatten().map(|b| b.housing()).sum()
    }

    pub fn build(&mut self, map: &Map, i: usize, b: Building) -> Result<(), BuildError> {
        let t = map.terrain(i).ok_or(BuildError::OutOfBounds)?;
        if self.grid[i].is_some() {
            return Err(BuildError::Occupied);
        }
        if t == Terrain::Water {
            return Err(BuildError::Unbuildable);
        }
        if b.terrain().is_some_and(|need| need != t) {
            return Err(BuildError::WrongTerrain);
        }
        if !self.stock.pay(&b.cost()) {
            return Err(BuildError::NoResources);
        }
        self.grid[i] = Some(b);
        Ok(())
    }

    /// Ricerca: 20 di conoscenza sbloccano l'irrigazione (+2 cibo per fattoria).
    pub fn research_irrigation(&mut self) -> bool {
        if self.irrigation || self.knowledge < IRRIGATION_COST {
            return false;
        }
        self.knowledge -= IRRIGATION_COST;
        self.irrigation = true;
        true
    }

    /// Produzione, consumo, carestia e crescita di un turno.
    fn tick(&mut self) {
        for b in self.grid.iter().flatten() {
            match b {
                Building::Farm => self.stock.food += if self.irrigation { 6 } else { 4 },
                Building::Lumber => self.stock.wood += 3,
                Building::Quarry => self.stock.stone += 2,
                Building::Market => self.stock.gold += 3,
                Building::Library => self.knowledge += 2,
                Building::House => {}
            }
        }
        // Ogni due cittadini consumano una unità di cibo per turno.
        self.stock.food -= (self.citizens as i32 + 1) / 2;
        if self.stock.food < 0 {
            self.stock.food = 0;
            self.citizens = self.citizens.saturating_sub(1);
        } else if self.citizens < self.capacity() && self.stock.food >= 2 {
            self.citizens += 1;
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    Playing,
    Won,
    Lost,
}

pub struct Game {
    pub map: Map,
    pub player: Empire,
    pub rival: Empire,
    pub turns: u32,
}

impl Game {
    /// Due imperi su una mappa generata: il giocatore a ovest, l'IA a est.
    pub fn new(w: usize, h: usize, seed: u64) -> Self {
        assert!(w >= 8 && h >= 4, "mappa troppo piccola");
        let mut map = Map::generate(w, h, seed);
        let (px, py) = (w / 4, h / 2);
        let (rx, ry) = (3 * w / 4, h / 2);
        map.clear_area(px, py);
        map.clear_area(rx, ry);
        let stock = Stock::new(30, 40, 10, 0);
        let mut player = Empire::new(&map, stock);
        let mut rival = Empire::new(&map, stock);
        // Ogni impero parte con una casa gratuita.
        let (pi, ri) = (map.idx(px, py), map.idx(rx, ry));
        player.grid[pi] = Some(Building::House);
        rival.grid[ri] = Some(Building::House);
        Game {
            map,
            player,
            rival,
            turns: 0,
        }
    }

    pub fn end_turn(&mut self) {
        if self.status() != Status::Playing {
            return;
        }
        self.player.tick();
        self.rival.tick();
        self.turns += 1;
        ai_turn(&mut self.rival, &self.map);
    }

    /// Vince chi raggiunge per primo `WIN_POP`; perde chi si estingue o
    /// lascia che l'IA lo superi. In caso di parità vince il giocatore.
    pub fn status(&self) -> Status {
        if self.player.citizens >= WIN_POP {
            Status::Won
        } else if self.rival.citizens >= WIN_POP || self.player.citizens == 0 {
            Status::Lost
        } else {
            Status::Playing
        }
    }
}

/// IA semplice e deterministica: al massimo una costruzione per turno,
/// nell'ordine di bisogno. Ricerca l'irrigazione appena possibile.
fn ai_turn(e: &mut Empire, map: &Map) {
    if e.research_irrigation() {
        return;
    }
    let plan = [
        (Building::House, e.capacity() <= e.citizens),
        (
            Building::Farm,
            e.count(Building::Farm) < 2 + e.citizens as usize / 6,
        ),
        (Building::Lumber, e.count(Building::Lumber) < 2),
        (Building::Quarry, e.count(Building::Quarry) < 1),
        (Building::Library, e.count(Building::Library) < 1),
        (Building::Market, e.count(Building::Market) < 2),
    ];
    for (b, wanted) in plan {
        if !wanted {
            continue;
        }
        if let Some(i) = find_spot(e, map, b) {
            if e.build(map, i, b).is_ok() {
                return;
            }
        }
    }
}

fn find_spot(e: &Empire, map: &Map, b: Building) -> Option<usize> {
    (0..map.w * map.h).find(|&i| {
        e.building_at(i).is_none()
            && map
                .terrain(i)
                .is_some_and(|t| t != Terrain::Water && b.terrain().is_none_or(|need| need == t))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grass(w: usize, h: usize) -> Map {
        Map::filled(w, h, Terrain::Grass)
    }

    #[test]
    fn build_deducts_cost_and_places_building() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 20, 0, 0));
        assert_eq!(e.build(&map, 0, Building::Farm), Ok(()));
        assert_eq!(e.stock.wood, 15);
        assert_eq!(e.building_at(0), Some(Building::Farm));
    }

    #[test]
    fn build_without_resources_changes_nothing() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 4, 0, 0));
        assert_eq!(
            e.build(&map, 0, Building::Farm),
            Err(BuildError::NoResources)
        );
        assert_eq!(e.stock.wood, 4);
        assert_eq!(e.building_at(0), None);
    }

    #[test]
    fn build_respects_terrain_and_occupancy() {
        let mut map = grass(4, 4);
        map.terrain[1] = Terrain::Water;
        map.terrain[2] = Terrain::Mountain;
        let mut e = Empire::new(&map, Stock::new(0, 100, 100, 100));
        assert_eq!(
            e.build(&map, 1, Building::House),
            Err(BuildError::Unbuildable)
        );
        assert_eq!(
            e.build(&map, 2, Building::Farm),
            Err(BuildError::WrongTerrain)
        );
        assert_eq!(e.build(&map, 0, Building::House), Ok(()));
        assert_eq!(e.build(&map, 0, Building::House), Err(BuildError::Occupied));
        assert_eq!(
            e.build(&map, 99, Building::House),
            Err(BuildError::OutOfBounds)
        );
    }

    #[test]
    fn farm_produces_food() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 5, 0, 0));
        e.citizens = 0;
        e.build(&map, 0, Building::Farm).unwrap();
        e.tick();
        assert_eq!(e.stock.food, 4);
    }

    #[test]
    fn irrigation_boosts_farms() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 5, 0, 0));
        e.citizens = 0;
        e.build(&map, 0, Building::Farm).unwrap();
        e.knowledge = IRRIGATION_COST;
        assert!(e.research_irrigation());
        assert!(!e.research_irrigation());
        e.tick();
        assert_eq!(e.stock.food, 6);
        assert_eq!(e.knowledge, 0);
    }

    #[test]
    fn starvation_shrinks_population() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 0, 0, 0));
        e.citizens = 4; // consuma 2 cibo, ne ha 0
        e.tick();
        assert_eq!(e.stock.food, 0);
        assert_eq!(e.citizens, 3);
    }

    #[test]
    fn population_grows_only_with_housing_and_food() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(10, 10, 0, 0));
        e.citizens = 0;
        e.tick();
        assert_eq!(e.citizens, 0, "nessuna casa, nessuna crescita");
        e.build(&map, 0, Building::House).unwrap();
        e.tick();
        assert_eq!(e.citizens, 1);
    }

    #[test]
    fn win_when_player_reaches_target() {
        let mut g = Game::new(16, 10, 1);
        assert_eq!(g.status(), Status::Playing);
        g.player.citizens = WIN_POP;
        assert_eq!(g.status(), Status::Won);
    }

    #[test]
    fn lose_when_rival_reaches_target_first() {
        let mut g = Game::new(16, 10, 1);
        g.rival.citizens = WIN_POP;
        assert_eq!(g.status(), Status::Lost);
    }

    #[test]
    fn lose_when_player_population_dies() {
        let mut g = Game::new(16, 10, 1);
        g.player.citizens = 0;
        assert_eq!(g.status(), Status::Lost);
    }

    #[test]
    fn game_stops_after_it_ends() {
        let mut g = Game::new(16, 10, 1);
        g.player.citizens = WIN_POP;
        g.end_turn();
        assert_eq!(g.turns, 0);
    }

    #[test]
    fn ai_builds_and_game_runs_without_panic() {
        let mut g = Game::new(24, 12, 0x4841494B55);
        for _ in 0..400 {
            g.end_turn();
            if g.status() != Status::Playing {
                break;
            }
        }
        assert!(g.rival.count(Building::House) > 0);
        assert!(g.rival.count(Building::Farm) > 0);
    }

    /// Misura del costo di un turno su mappa grande:
    /// `cargo test -p citybuilder --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn perf_end_turn() {
        // Si salta `status()`: il costo va misurato anche a partita finita.
        let mut g = Game::new(64, 64, 7);
        let start = std::time::Instant::now();
        let n = 2000;
        for _ in 0..n {
            g.player.tick();
            g.rival.tick();
            ai_turn(&mut g.rival, &g.map);
        }
        println!("turno medio: {:?}", start.elapsed() / n);
    }
}
