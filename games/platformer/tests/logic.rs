use platformer::levels::LEVELS;
use platformer::*;

fn custom(src: &'static str) -> Game {
    assert!(Level::parse(src).is_some(), "livello valido");
    Game::from_src(src)
}

fn run(g: &mut Game, input: u8, frames: usize) {
    for _ in 0..frames {
        g.update(input);
    }
}

#[test]
fn levels_parse_and_are_rectangular() {
    for src in LEVELS {
        let rows: Vec<&str> = src.lines().filter(|r| !r.is_empty()).collect();
        let lvl = Level::parse(src).expect("parse");
        assert_eq!(lvl.h, 15);
        assert!(
            rows.iter().all(|r| r.chars().count() == lvl.w),
            "righe di lunghezza diversa"
        );
        let flags = rows
            .iter()
            .flat_map(|r| r.chars())
            .filter(|&c| c == 'F')
            .count();
        assert!(flags > 0, "manca la bandiera");
    }
}

#[test]
fn rejects_bad_characters() {
    assert!(Level::parse("P.x\n###").is_none());
    assert!(Level::parse("..\n##").is_none()); // nessuna partenza
}

#[test]
fn player_lands_on_ground() {
    let mut g = custom("P.\n..\n##");
    run(&mut g, 0, 60);
    assert!(g.p.on_ground);
    assert_eq!(g.p.b.y + g.p.b.h * SUB, 32 * SUB);
}

#[test]
fn wall_stops_player() {
    let mut g = custom("P...#\n....#\n#####");
    run(&mut g, BTN_RIGHT | BTN_RUN, 120);
    assert!(g.p.b.x / SUB + g.p.b.w <= 64);
    assert_eq!(g.p.b.vx, 0);
}

#[test]
fn coin_is_collected() {
    let mut g = custom("P.\nc.\n##");
    run(&mut g, 0, 60);
    assert_eq!(g.coins, 1);
    assert_eq!(g.score, 100);
    assert_eq!(g.lvl.tile(0, 1), Tile::Air);
}

#[test]
fn question_block_gives_coin_once() {
    let mut g = custom("?.\nP.\n##");
    run(&mut g, 0, 10);
    g.update(BTN_JUMP);
    run(&mut g, BTN_JUMP, 25);
    run(&mut g, 0, 5);
    assert_eq!(g.lvl.tile(0, 0), Tile::Used);
    assert_eq!(g.coins, 1);
}

#[test]
fn stomping_fungo_does_not_cost_life() {
    let mut g = custom("P..\n...\ng..\n###");
    g.p.b.x = 0;
    run(&mut g, 0, 200);
    assert_eq!(g.lives, LIVES_START);
    assert!(g.score >= 100 || g.mobs.iter().all(|m| !m.alive));
}

#[test]
fn falling_in_pit_costs_life() {
    let mut g = custom("P.\n..\n..\n..\n.#");
    run(&mut g, 0, 25);
    assert_eq!(g.lives, LIVES_START - 1);
}

#[test]
fn bot_plays_every_level_without_panic() {
    for src in LEVELS {
        let mut g = Game::from_src(src);
        let mut bot = platformer::bot::Bot::default();
        for _ in 0..3_000 {
            let input = bot.input(&g);
            g.update(input);
            platformer::render::draw(&g, &mut vec![0u32; 256 * 240], 256, 240);
        }
    }
}

#[test]
fn map_moves_to_next_level_after_clear() {
    let mut g = Game::new();
    g.update(BTN_START); // entra nel livello 1
    assert_eq!(g.state, State::Playing);
    g.level_clear_for_test();
    run(&mut g, 0, 200);
    assert_eq!(g.state, State::Map);
    assert_eq!((g.cursor, g.unlocked), (1, 2));
}
