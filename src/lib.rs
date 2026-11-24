//! treefaller: al romper un tronco de un árbol natural, lo tala de arriba hacia abajo.
//! Las hojas del árbol se rompen con él y sueltan su botín.
//! `/treefaller anim on|off` activa o desactiva la animación.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use mod_api::{
    self, BlockState, Command, Flow, Hand, Hooks, LevelRef, PlayerDestroyedBlockEvent, PlayerRef,
    RegisterCommandsEvent, RunModCommandEvent, ScheduledTaskEvent, ServerHost,
};

const MAX_LOGS: usize = 256; // más troncos que esto: no se tala
const RADIUS: i32 = 12; // distancia horizontal máxima desde el tronco roto
const MAX_HEIGHT: i32 = 40; // altura máxima sobre el tronco roto
const MIN_LEAVES: usize = 5; // hojas naturales mínimas: evita talar construcciones
const MAX_LEAVES: usize = 2000; // hojas máximas que se rompen
const LEAF_REACH: i32 = 6; // distancia máxima de una hoja a un tronco
const MAX_SCAN: i32 = 60_000; // bloques máximos a mirar buscando troncos de otros árboles
const LAYER_DELAY_TICKS: u32 = 2; // ticks entre una capa y la siguiente
const DIMENSION: &str = "minecraft:overworld"; // los árboles con hojas crecen aquí

const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Lo que se retira en una misma altura: troncos y hojas.
#[derive(Default)]
struct Step {
    logs: Vec<[i32; 3]>,
    leaves: Vec<[i32; 3]>,
}

/// Una tala en marcha: pasos de arriba hacia abajo, y cuál toca ahora.
struct Job {
    player: PlayerRef,
    steps: Vec<Step>,
    next: usize,
}

struct Mod {
    busy: AtomicBool,
    animate: AtomicBool,
    next_id: AtomicU64,
    jobs: Mutex<HashMap<u64, Job>>,
}

impl Default for Mod {
    fn default() -> Self {
        Self {
            busy: AtomicBool::new(false),
            animate: AtomicBool::new(true),
            next_id: AtomicU64::new(0),
            jobs: Mutex::new(HashMap::new()),
        }
    }
}

impl Mod {
    fn jobs(&self) -> std::sync::MutexGuard<'_, HashMap<u64, Job>> {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn player_destroyed_block(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerDestroyedBlockEvent,
    ) {
        // Nuestras propias roturas vuelven a pasar por aquí: no las procesamos otra vez.
        if self.busy.swap(true, Ordering::Acquire) {
            return;
        }
        if let Some(mut job) = plan(host, event.pos, event.state) {
            if self.animate.load(Ordering::Relaxed) {
                let id = self.next_id.fetch_add(1, Ordering::Relaxed);
                self.jobs().insert(id, job);
                if host.schedule_task(LAYER_DELAY_TICKS, id).is_err() {
                    self.jobs().remove(&id);
                }
            } else {
                // Sin animación: todo de golpe.
                while job.next < job.steps.len() {
                    run_step(host, &mut job);
                }
            }
        }
        self.busy.store(false, Ordering::Release);
    }

    fn scheduled_task(&self, host: &mut dyn ServerHost, event: &mut ScheduledTaskEvent<'_>) {
        let id = event.token;
        let Some(mut job) = self.jobs().remove(&id) else {
            return;
        };
        if job.next >= job.steps.len() || !holds_axe(host, &job.player) {
            return; // terminó, o el hacha ya no está en la mano
        }
        self.busy.store(true, Ordering::Release);
        run_step(host, &mut job);
        self.busy.store(false, Ordering::Release);
        if job.next < job.steps.len() {
            self.jobs().insert(id, job);
            if host.schedule_task(LAYER_DELAY_TICKS, id).is_err() {
                self.jobs().remove(&id);
            }
        }
    }

    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new("treefaller anim on"));
        event.commands.push(Command::new("treefaller anim off"));
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let words = &event.command.literals;
        if words.first().map(String::as_str) != Some("treefaller") {
            return Flow::Continue;
        }
        let on = match words.get(2).map(String::as_str) {
            Some("on") => true,
            Some("off") => false,
            _ => return Flow::Continue,
        };
        self.animate.store(on, Ordering::Relaxed);
        let text = if on {
            "animación activada"
        } else {
            "animación desactivada"
        };
        host.commands()
            .run_command(&format!("tellraw @a {{\"text\":\"treefaller: {text}\"}}"));
        Flow::Cancel
    }
}

/// Retira un paso: rompe sus troncos como el jugador y sus hojas con su botín.
fn run_step(host: &mut dyn ServerHost, job: &mut Job) {
    let step = std::mem::take(&mut job.steps[job.next]);
    job.next += 1;
    for pos in step.logs {
        job.player.destroy_block(host, pos, false);
    }
    for [x, y, z] in step.leaves {
        // `destroy` rompe el bloque con partículas y suelta su botín, sin gastar el hacha.
        host.commands().run_command(&format!(
            "execute in {DIMENSION} run setblock {x} {y} {z} minecraft:air destroy"
        ));
    }
}

/// Decide si hay que talar; devuelve la tala planificada, de arriba hacia abajo.
fn plan(host: &mut dyn ServerHost, origin: [i32; 3], state: BlockState) -> Option<Job> {
    let player = host.players().acting_player()?;
    let name = host.block_state_string(state)?;
    if !is_log(&name) || is_sneaking(host, &player) || !holds_axe(host, &player) {
        return None;
    }
    let level = player.level(host)?;
    let (logs, leaf_count) = find_tree(host, level, origin);
    if logs.is_empty() || leaf_count < MIN_LEAVES {
        return None;
    }
    let mut ours = logs.clone();
    ours.push(origin);

    let mut by_height: BTreeMap<i32, Step> = BTreeMap::new();
    for pos in &logs {
        by_height.entry(pos[1]).or_default().logs.push(*pos);
    }
    for pos in find_leaves(host, level, &ours) {
        by_height.entry(pos[1]).or_default().leaves.push(pos);
    }
    let steps: Vec<Step> = by_height.into_values().rev().collect(); // de arriba hacia abajo
    Some(Job {
        player,
        steps,
        next: 0,
    })
}

/// Distancias (en pasos a través de hojas) desde `seeds` a cada hoja natural de `allowed`
/// (o de cualquiera, si es `None`), hasta `LEAF_REACH`.
fn leaf_distances(
    host: &dyn ServerHost,
    level: LevelRef,
    seeds: &[[i32; 3]],
    allowed: Option<&HashMap<[i32; 3], i32>>,
) -> HashMap<[i32; 3], i32> {
    let mut dist: HashMap<[i32; 3], i32> = HashMap::new();
    let mut queue: VecDeque<([i32; 3], i32)> = seeds.iter().map(|p| (*p, 0)).collect();
    while let Some((at, d)) = queue.pop_front() {
        if d >= LEAF_REACH {
            continue;
        }
        for f in FACES {
            let p = shift(at, f);
            if dist.contains_key(&p) {
                continue;
            }
            let is_leaf = match allowed {
                Some(set) => set.contains_key(&p),
                None => name_at(host, level, p).is_some_and(|n| is_natural_leaves(&n)),
            };
            if !is_leaf {
                continue;
            }
            dist.insert(p, d + 1);
            if allowed.is_none() && dist.len() >= MAX_LEAVES {
                return dist;
            }
            queue.push_back((p, d + 1));
        }
    }
    dist
}

/// Hojas naturales de este árbol: las que están más cerca de sus troncos que de los de otro árbol.
fn find_leaves(host: &dyn ServerHost, level: LevelRef, ours: &[[i32; 3]]) -> Vec<[i32; 3]> {
    let mine = leaf_distances(host, level, ours, None);
    if mine.is_empty() {
        return Vec::new();
    }
    // Troncos de otros árboles dentro de la zona que ocupan estas hojas.
    let mut lo = [i32::MAX; 3];
    let mut hi = [i32::MIN; 3];
    for p in mine.keys() {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i] - 1);
            hi[i] = hi[i].max(p[i] + 1);
        }
    }
    let volume =
        (hi[0] - lo[0] + 1) as i64 * (hi[1] - lo[1] + 1) as i64 * (hi[2] - lo[2] + 1) as i64;
    let own: HashSet<[i32; 3]> = ours.iter().copied().collect();
    let mut foreign: Vec<[i32; 3]> = Vec::new();
    if volume <= MAX_SCAN as i64 {
        for x in lo[0]..=hi[0] {
            for y in lo[1]..=hi[1] {
                for z in lo[2]..=hi[2] {
                    let p = [x, y, z];
                    if !own.contains(&p) && name_at(host, level, p).is_some_and(|n| is_log(&n)) {
                        foreign.push(p);
                    }
                }
            }
        }
    }
    let theirs = if foreign.is_empty() {
        HashMap::new()
    } else {
        leaf_distances(host, level, &foreign, Some(&mine))
    };
    mine.into_iter()
        .filter(|(p, d)| theirs.get(p).is_none_or(|other| d < other))
        .map(|(p, _)| p)
        .collect()
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
            if !leaves.contains(&p)
                && name_at(host, level, p).is_some_and(|n| is_natural_leaves(&n))
            {
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
    let Some(living) = host.living(player.entity()) else {
        return false;
    };
    let Some(stack) = living.hand_item(host, Hand::Main) else {
        return false;
    };
    let held = stack.item();
    AXES.iter().any(|id| {
        mod_api::Identifier::parse(id)
            .ok()
            .and_then(|i| host.item_key(&i))
            == Some(held)
    })
}
