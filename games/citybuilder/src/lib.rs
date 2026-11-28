//! Città-impero: logica di gioco pura, senza I/O né rendering.
//! Regole e valori: vedi `README.md`. Il rendering vive in `view` e `raster`.

pub mod menu;
pub mod raster;
pub mod view;

/// Cittadini necessari per vincere.
pub const WIN_POP: u32 = 50;
/// Conoscenza per la ricerca dell'irrigazione.
const IRRIGATION_COST: i32 = 20;
/// Cibo massimo in magazzino: oltre, la produzione si perde.
const FOOD_CAP: i32 = 200;

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

    /// Paga `c` solo se le risorse bastano e il costo non è negativo: mai uno stato parziale.
    pub fn pay(&mut self, c: &Stock) -> bool {
        if c.food < 0 || c.wood < 0 || c.stone < 0 || c.gold < 0 || !self.covers(c) {
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
            Building::Library => Stock::new(0, 10, 10, 5),
        }
    }

    /// Terreno obbligatorio; `None` vuol dire qualsiasi terreno non acquatico.
    pub fn terrain(self) -> Option<Terrain> {
        match self {
            Building::House | Building::Farm => Some(Terrain::Grass),
            Building::Lumber => Some(Terrain::Forest),
            Building::Quarry => Some(Terrain::Mountain),
            Building::Market | Building::Library => None,
        }
    }

    /// Cittadini che l'edificio ospita.
    fn housing(self) -> u32 {
        match self {
            Building::House => 5,
            _ => 0,
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
    /// Generatore deterministico: stesso (w, h, seme), stessa mappa. Il seme passa da splitmix64,
    /// così seed adiacenti danno mappe diverse; poi xorshift64 per i terreni.
    pub fn generate(w: usize, h: usize, seed: u64) -> Self {
        let mut s = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        s = (s ^ (s >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        s = (s ^ (s >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        s ^= s >> 31;
        s |= 1;
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BuildError {
    OutOfBounds,
    TurnLimit,
    Occupied,
    Unbuildable,
    WrongTerrain,
    NoResources,
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::OutOfBounds => write!(f, "fuori dalla mappa"),
            BuildError::TurnLimit => write!(f, "un edificio per turno: passa il turno con t"),
            BuildError::Occupied => write!(f, "casella già occupata"),
            BuildError::Unbuildable => write!(f, "l'acqua non si costruisce"),
            BuildError::WrongTerrain => write!(f, "terreno sbagliato per questo edificio"),
            BuildError::NoResources => write!(f, "risorse insufficienti"),
        }
    }
}

/// Un impero: risorse, popolazione e le sue costruzioni sulla mappa condivisa.
#[derive(Clone, Debug)]
pub struct Empire {
    pub stock: Stock,
    pub knowledge: i32,
    pub citizens: u32,
    pub irrigation: bool,
    /// Costruzione già fatta in questo turno: al massimo una per impero.
    built_this_turn: bool,
    grid: Vec<Option<Building>>,
}

impl Empire {
    pub fn new(map: &Map, stock: Stock) -> Self {
        Empire {
            stock,
            knowledge: 0,
            citizens: 2,
            irrigation: false,
            built_this_turn: false,
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

    /// Costruisce in `i` (ordine dei controlli: mappa, turno, casella, acqua, terreno, risorse).
    pub fn build(&mut self, map: &Map, i: usize, b: Building) -> Result<(), BuildError> {
        let t = map.terrain(i).ok_or(BuildError::OutOfBounds)?;
        if i >= self.grid.len() {
            return Err(BuildError::OutOfBounds);
        }
        if self.built_this_turn {
            return Err(BuildError::TurnLimit);
        }
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
        self.built_this_turn = true;
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

    /// Un turno di produzione, consumo e crescita.
    fn tick(&mut self) {
        for b in self.grid.iter().flatten() {
            match b {
                Building::Farm => {
                    let yield_ = if self.irrigation { 6 } else { 4 };
                    self.stock.food = (self.stock.food + yield_).min(FOOD_CAP);
                }
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
            // +1 a turno, +1 ogni tre fattorie, massimo +3; mai oltre la capienza.
            let growth = (1 + self.count(Building::Farm) as u32 / 3).min(3);
            self.citizens = (self.citizens + growth).min(self.capacity());
        }
        self.built_this_turn = false;
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    Playing,
    Won,
    Lost,
    Draw,
}

pub struct Game {
    pub map: Map,
    pub player: Empire,
    pub rival: Empire,
    pub turns: u32,
    /// Cursore del giocatore (x, y): dove agisce `b <edificio>`.
    pub cursor: (usize, usize),
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
        player.grid[map.idx(px, py)] = Some(Building::House);
        rival.grid[map.idx(rx, ry)] = Some(Building::House);
        Game {
            map,
            player,
            rival,
            turns: 0,
            cursor: (px, py),
        }
    }

    /// Sposta il cursore di un passo, senza uscire dalla mappa.
    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let (x, y) = self.cursor;
        let nx = (x as i32 + dx).clamp(0, self.map.w as i32 - 1) as usize;
        let ny = (y as i32 + dy).clamp(0, self.map.h as i32 - 1) as usize;
        self.cursor = (nx, ny);
    }

    /// Fine turno: entrambi producono, poi l'IA costruisce.
    pub fn end_turn(&mut self) {
        if self.status() != Status::Playing {
            return;
        }
        self.player.tick();
        self.rival.tick();
        self.turns += 1;
        ai_turn(&mut self.rival, &self.map);
    }

    /// Vince chi arriva a `WIN_POP`; se entrambi ci arrivano nello stesso turno è patta.
    /// Perde chi si estingue o lascia che l'IA arrivi prima.
    pub fn status(&self) -> Status {
        match (
            self.player.citizens >= WIN_POP,
            self.rival.citizens >= WIN_POP,
        ) {
            (true, true) => Status::Draw,
            (true, false) => Status::Won,
            (false, true) => Status::Lost,
            _ if self.player.citizens == 0 => Status::Lost,
            _ => Status::Playing,
        }
    }
}

/// IA semplice e deterministica: al massimo una costruzione per turno, nell'ordine di bisogno,
/// sempre nella casella libera più vicina alla propria capitale (regole uguali al giocatore).
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
    let capital = (3 * map.w / 4, map.h / 2);
    for (b, wanted) in plan {
        if !wanted {
            continue;
        }
        if let Some(i) = find_spot(e, map, b, capital) {
            if e.build(map, i, b).is_ok() {
                return;
            }
        }
    }
}

/// Casella libera e adatta a `b`, la più vicina a `near`.
fn find_spot(e: &Empire, map: &Map, b: Building, near: (usize, usize)) -> Option<usize> {
    (0..map.w * map.h)
        .filter(|&i| {
            e.building_at(i).is_none()
                && map.terrain(i).is_some_and(|t| {
                    t != Terrain::Water && b.terrain().is_none_or(|need| need == t)
                })
        })
        .min_by_key(|&i| {
            let (x, y) = (i % map.w, i / map.w);
            x.abs_diff(near.0) + y.abs_diff(near.1)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grass(w: usize, h: usize) -> Map {
        Map {
            w,
            h,
            terrain: vec![Terrain::Grass; w * h],
        }
    }

    #[test]
    fn build_pays_and_places() {
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
    fn terrain_water_and_occupancy() {
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
        e.built_this_turn = false;
        assert_eq!(e.build(&map, 0, Building::House), Err(BuildError::Occupied));
        assert_eq!(
            e.build(&map, 99, Building::House),
            Err(BuildError::OutOfBounds)
        );
    }

    #[test]
    fn house_only_on_grass() {
        let mut map = grass(4, 4);
        map.terrain[0] = Terrain::Forest;
        let mut e = Empire::new(&map, Stock::new(0, 100, 0, 0));
        assert_eq!(
            e.build(&map, 0, Building::House),
            Err(BuildError::WrongTerrain)
        );
    }

    #[test]
    fn un_edificio_per_turno() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 100, 0, 0));
        assert_eq!(e.build(&map, 0, Building::Farm), Ok(()));
        assert_eq!(e.build(&map, 1, Building::Farm), Err(BuildError::TurnLimit));
        e.tick();
        assert_eq!(e.build(&map, 1, Building::Farm), Ok(()));
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
    fn irrigation_needs_knowledge_and_runs_once() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 5, 0, 0));
        assert!(!e.research_irrigation());
        e.knowledge = 20;
        assert!(e.research_irrigation());
        assert_eq!(e.knowledge, 0);
        assert!(!e.research_irrigation());
    }

    #[test]
    fn starvation_shrinks_population() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(0, 0, 0, 0));
        e.citizens = 4;
        e.tick();
        assert_eq!(e.citizens, 3);
    }

    #[test]
    fn growth_needs_housing_food_and_farms() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(10, 0, 0, 0));
        e.citizens = 1;
        e.tick();
        assert_eq!(e.citizens, 1, "senza casa non cresce");
        e.grid[0] = Some(Building::House);
        e.tick();
        assert_eq!(e.citizens, 2);
        // Tre fattorie: crescita +2 a turno.
        for i in 1..=3 {
            e.grid[i] = Some(Building::Farm);
        }
        e.grid[4] = Some(Building::House);
        e.stock.food = 50;
        e.tick();
        assert_eq!(e.citizens, 4);
    }

    #[test]
    fn food_is_capped() {
        let map = grass(4, 4);
        let mut e = Empire::new(&map, Stock::new(FOOD_CAP, 0, 0, 0));
        e.grid[0] = Some(Building::Farm);
        e.tick();
        assert_eq!(e.stock.food, FOOD_CAP - 1);
    }

    #[test]
    fn pay_rejects_negative_costs() {
        let mut s = Stock::new(0, 0, 0, 0);
        assert!(!s.pay(&Stock::new(0, -5, 0, 0)));
        assert_eq!(s.wood, 0);
    }

    #[test]
    fn win_lose_draw() {
        let mut g = Game::new(8, 4, 1);
        g.player.citizens = WIN_POP;
        assert_eq!(g.status(), Status::Won);
        g.rival.citizens = WIN_POP;
        assert_eq!(g.status(), Status::Draw);
        g.player.citizens = 3;
        g.rival.citizens = WIN_POP;
        assert_eq!(g.status(), Status::Lost);
    }

    #[test]
    fn game_stops_after_it_ends() {
        let mut g = Game::new(8, 4, 1);
        g.player.citizens = WIN_POP;
        let t = g.turns;
        g.end_turn();
        assert_eq!(g.turns, t);
    }

    #[test]
    fn cursor_stays_on_map() {
        let mut g = Game::new(8, 4, 1);
        for _ in 0..20 {
            g.move_cursor(1, 1);
        }
        assert_eq!(g.cursor, (7, 3));
    }

    #[test]
    fn map_is_deterministic_and_adjacent_seeds_differ() {
        let a = Map::generate(24, 12, 0);
        let b = Map::generate(24, 12, 0);
        let c = Map::generate(24, 12, 1);
        assert_eq!(a.terrain, b.terrain);
        assert_ne!(a.terrain, c.terrain);
    }

    #[test]
    fn ai_builds_and_game_runs_without_panic() {
        let mut g = Game::new(24, 12, 7);
        for _ in 0..60 {
            g.end_turn();
        }
        assert!(g.rival.count(Building::House) > 0);
        assert!(g.rival.citizens > 2);
    }

    #[test]
    fn build_out_of_range_is_error_not_panic() {
        let small = grass(4, 4);
        let big = grass(8, 8);
        let mut e = Empire::new(&small, Stock::new(0, 100, 0, 0));
        assert_eq!(
            e.build(&big, 40, Building::House),
            Err(BuildError::OutOfBounds)
        );
    }
}
