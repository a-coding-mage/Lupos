//! linux-parity: partial
//! linux-source: vendor/linux/block/bfq-wf2q.c
//! test-origin: linux:vendor/linux/block/bfq-wf2q.c
//! B-WF2Q+ timestamp and weight helpers used by BFQ.

pub const WFQ_SERVICE_SHIFT: u32 = 22;
pub const IOPRIO_NR_LEVELS: u16 = 8;
pub const BFQ_WEIGHT_CONVERSION_COEFF: u16 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BfqEntity {
    pub start: u64,
    pub finish: u64,
    pub min_start: u64,
    pub service: i32,
    pub budget: i32,
    pub weight: u16,
    pub new_weight: u16,
    pub orig_weight: u16,
    pub prio_changed: bool,
    pub on_st_or_in_serv: bool,
}

impl BfqEntity {
    pub const fn new(weight: u16, budget: i32) -> Self {
        Self {
            start: 0,
            finish: 0,
            min_start: 0,
            service: 0,
            budget,
            weight,
            new_weight: weight,
            orig_weight: weight,
            prio_changed: false,
            on_st_or_in_serv: false,
        }
    }

    pub fn calc_finish(&mut self, service: u64) {
        self.finish = self
            .start
            .saturating_add(bfq_delta(service, self.weight as u64));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BfqServiceTree {
    pub vtime: u64,
    pub wsum: u64,
    active: alloc::vec::Vec<BfqEntity>,
    idle: alloc::vec::Vec<BfqEntity>,
}

extern crate alloc;

impl BfqServiceTree {
    pub const fn new() -> Self {
        Self {
            vtime: 0,
            wsum: 0,
            active: alloc::vec::Vec::new(),
            idle: alloc::vec::Vec::new(),
        }
    }

    pub fn insert_active(&mut self, mut entity: BfqEntity) {
        entity.min_start = entity.start;
        entity.on_st_or_in_serv = true;
        self.wsum = self.wsum.saturating_add(entity.weight as u64);
        let pos = self
            .active
            .binary_search_by(|probe| probe.finish.cmp(&entity.finish))
            .unwrap_or_else(|pos| pos);
        self.active.insert(pos, entity);
        self.update_min_start();
    }

    pub fn pop_next(&mut self) -> Option<BfqEntity> {
        let index = self
            .active
            .iter()
            .enumerate()
            .filter(|(_, entity)| !bfq_gt(entity.start, self.vtime))
            .min_by_key(|(_, entity)| entity.finish)
            .map(|(index, _)| index)?;
        let entity = self.active.remove(index);
        self.wsum = self.wsum.saturating_sub(entity.weight as u64);
        self.update_min_start();
        Some(entity)
    }

    pub fn insert_idle(&mut self, entity: BfqEntity) {
        let pos = self
            .idle
            .binary_search_by(|probe| probe.finish.cmp(&entity.finish))
            .unwrap_or_else(|pos| pos);
        self.idle.insert(pos, entity);
    }

    pub fn first_idle(&self) -> Option<&BfqEntity> {
        self.idle.first()
    }

    pub fn last_idle(&self) -> Option<&BfqEntity> {
        self.idle.last()
    }

    pub fn active_len(&self) -> usize {
        self.active.len()
    }

    fn update_min_start(&mut self) {
        let min_start = self
            .active
            .iter()
            .map(|entity| entity.start)
            .min()
            .unwrap_or(0);
        for entity in &mut self.active {
            entity.min_start = min_start.min(entity.start);
        }
    }
}

impl Default for BfqServiceTree {
    fn default() -> Self {
        Self::new()
    }
}

pub fn bfq_gt(a: u64, b: u64) -> bool {
    (a as i64).wrapping_sub(b as i64) > 0
}

pub fn bfq_delta(service: u64, weight: u64) -> u64 {
    if weight == 0 {
        return u64::MAX;
    }
    (service << WFQ_SERVICE_SHIFT) / weight
}

pub fn bfq_ioprio_to_weight(ioprio: u16) -> u16 {
    IOPRIO_NR_LEVELS
        .saturating_sub(ioprio)
        .saturating_mul(BFQ_WEIGHT_CONVERSION_COEFF)
}

pub fn bfq_weight_to_ioprio(weight: u16) -> u16 {
    IOPRIO_NR_LEVELS.saturating_sub(weight / BFQ_WEIGHT_CONVERSION_COEFF)
}

pub fn bfq_bfqq_served(entity: &mut BfqEntity, st: &mut BfqServiceTree, served: i32) {
    if served <= 0 {
        return;
    }
    entity.service = entity.service.saturating_add(served);
    if st.wsum != 0 {
        st.vtime = st.vtime.saturating_add(bfq_delta(served as u64, st.wsum));
    }
}

pub fn bfq_bfqq_charge_time(
    entity: &mut BfqEntity,
    max_budget: i32,
    timeout_ms: u64,
    time_ms: u64,
) -> i32 {
    if timeout_ms == 0 {
        return 0;
    }
    let bounded = time_ms.min(timeout_ms);
    let service_for_time = (max_budget as u64).saturating_mul(bounded) / timeout_ms;
    let total = (service_for_time as i32).max(entity.service);
    if total > entity.budget {
        entity.budget = total;
    }
    let charged = total.saturating_sub(entity.service);
    entity.service = entity.service.saturating_add(charged);
    charged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wf2q_weight_and_delta_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-wf2q.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/block/bfq-iosched.h"
        ));
        assert!(source.contains("#define WFQ_SERVICE_SHIFT\t22"));
        assert!(source.contains("(u64)service << WFQ_SERVICE_SHIFT"));
        assert!(source.contains("bfq_ioprio_to_weight(int ioprio)"));
        assert!(source.contains("(IOPRIO_NR_LEVELS - ioprio) * BFQ_WEIGHT_CONVERSION_COEFF"));
        assert!(header.contains("#define BFQ_WEIGHT_CONVERSION_COEFF\t10"));

        assert_eq!(bfq_ioprio_to_weight(0), 80);
        assert_eq!(bfq_ioprio_to_weight(4), 40);
        assert_eq!(bfq_weight_to_ioprio(40), 4);
        assert_eq!(bfq_delta(80, 40), 8_388_608);
    }

    #[test]
    fn service_tree_picks_eligible_lowest_finish_entity() {
        let mut st = BfqServiceTree::new();
        st.vtime = 100;
        let mut late = BfqEntity::new(40, 100);
        late.start = 200;
        late.finish = 220;
        let mut eligible = BfqEntity::new(20, 100);
        eligible.start = 80;
        eligible.finish = 180;
        st.insert_active(late);
        st.insert_active(eligible);

        assert_eq!(st.pop_next().unwrap().finish, 180);
        assert_eq!(st.active_len(), 1);
    }

    #[test]
    fn charge_time_inflates_slow_queue_service() {
        let mut entity = BfqEntity::new(40, 64);
        entity.service = 10;
        let charged = bfq_bfqq_charge_time(&mut entity, 128, 100, 50);
        assert_eq!(charged, 54);
        assert_eq!(entity.service, 64);
        assert_eq!(entity.budget, 64);
    }
}
