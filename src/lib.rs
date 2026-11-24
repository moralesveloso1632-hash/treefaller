//! treefaller: al romper un tronco de un árbol natural, tala el resto del árbol.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};

use mod_api::{self, BlockState, Hand, Hooks, LevelRef, PlayerDestroyedBlockEvent, PlayerRef, ServerHost, Text};

const MAX_LOGS: usize = 256; // más troncos que esto: no se tala
const RADIUS: i32 = 12; // distancia horizontal máxima desde el tronco roto
const MAX_HEIGHT: i32 = 40; // altura máxima sobre el tronco roto
const MIN_LEAVES: usize = 5; // hojas naturales mínimas: evita talar construcciones

const FACES: [[i32; 3]; 6] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

#[derive(Default)]
struct Mod {
    busy: AtomicBool,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn player_destroyed_block(&self, host: &mut dyn ServerHost, event: &mut PlayerDestroyedBlockEvent) {
        let has_player = host.players().acting_player().is_some();

        host.messages().broadcast(
    if has_player {
        Text::literal("TreeFeller TEST: acting_player = Some")
    } else {
        Text::literal("TreeFeller TEST: acting_player = None")
    }
);
    }
}

fn fell(host: &mut dyn ServerHost, origin: [i32; 3], state: BlockState) {
    let Some(player) = host.players().acting_player() else { return };
    let Some(name) = host.block_state_string(state) else { return };
    if !is_log(&name) || is_sneaking(host, &player) || !holds_axe(host, &player) {
        return;
    }
    let Some(level) = player.level(host) else { return };

    let (mut logs, leaves) = find_tree(host, level, origin);
    if logs.is_empty() || leaves < MIN_LEAVES {
        return;
    }
    logs.sort_by(|a, b| b[1].cmp(&a[1])); // de arriba hacia abajo
    for pos in logs {
        if !holds_axe(host, &player) {
            break; // el hacha se rompió
        }
        player.destroy_block(host, pos, false);
    }
}

/// Troncos conectados (en diagonal también) desde `origin` hacia arriba, y cuántas hojas naturales los tocan.
fn find_tree(host: &dyn ServerHost, level: LevelRef, origin: [i32; 3]) -> (Vec<[i32; 3]>, usize) {
    let mut seen: HashSet<[i32; 3]> = HashSet::from([origin]);
    let mut queue: VecDeque<[i32; 3]> = VecDeque::from([origin]);
    let mut logs: Vec<[i32; 3]> = Vec::new();
    let mut leaves: HashSet<[i32; 3]> = HashSet::new();

    while let Some(at) = queue.pop_front() {
        for d in FACES {
            let p = shift(at, d);
            if !leaves.contains(&p) && name_at(host, level, p).is_some_and(|n| is_natural_leaves(&n)) {
                leaves.insert(p);
            }
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let p = shift(at, [dx, dy, dz]);
                    if p[1] < origin[1]
                        || p[1] - origin[1] > MAX_HEIGHT
                        || (p[0] - origin[0]).abs() > RADIUS
                        || (p[2] - origin[2]).abs() > RADIUS
                    {
                        continue;
                    }
                    if !seen.insert(p) {
                        continue;
                    }
                    if name_at(host, level, p).is_some_and(|n| is_log(&n)) {
                        logs.push(p);
                        if logs.len() > MAX_LOGS {
                            return (Vec::new(), 0);
                        }
                        queue.push_back(p);
                    }
                }
            }
        }
    }
    (logs, leaves.len())
}

fn shift(a: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [a[0] + d[0], a[1] + d[1], a[2] + d[2]]
}

fn name_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    let state = level.block_state(host, pos)?;
    host.block_state_string(state)
}

fn block_id(state_name: &str) -> &str {
    state_name.split('[').next().unwrap_or(state_name)
}

fn is_log(state_name: &str) -> bool {
    let id = block_id(state_name);
    id.ends_with("_log") && !id.contains("stripped_")
}

fn is_natural_leaves(state_name: &str) -> bool {
    block_id(state_name).ends_with("_leaves") && state_name.contains("persistent=false")
}

fn is_sneaking(host: &dyn ServerHost, player: &PlayerRef) -> bool {
    // Si no se puede saber, lo tratamos como "agachado" para no talar por error.
    player.entity().is_shift_key_down(host).unwrap_or(true)
}

const AXES: [&str; 7] = [
    "minecraft:wooden_axe",
    "minecraft:stone_axe",
    "minecraft:copper_axe",
    "minecraft:iron_axe",
    "minecraft:golden_axe",
    "minecraft:diamond_axe",
    "minecraft:netherite_axe",
];

fn holds_axe(host: &dyn ServerHost, player: &PlayerRef) -> bool {
    let Some(living) = host.living(player.entity()) else { return false };
    let Some(stack) = living.hand_item(host, Hand::Main) else { return false };
    let held = stack.item();
    AXES.iter().any(|id| mod_api::Identifier::parse(id).ok().and_then(|i| host.item_key(&i)) == Some(held))
}
