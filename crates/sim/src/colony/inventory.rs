//! Slot-based inventories with the usual conveniences: stacking, moving,
//! splitting, sorting, quick stack and deposit. Used for players, chests,
//! robots and machine buffers.

use serde::{Deserialize, Serialize};

use super::items::{Category, Item};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stack {
    pub item: Item,
    pub count: u32,
    /// Favourites are skipped by sort, quick stack, deposit all and trash.
    pub fav: bool,
}

impl Stack {
    pub fn new(item: Item, count: u32) -> Stack {
        Stack {
            item,
            count,
            fav: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Stack>>,
}

/// Largest stack of `item` given a stack multiplier (the Power Lifters mod).
pub fn stack_limit(item: Item, mult: f32) -> u32 {
    let base = item.max_stack();
    if base <= 1 {
        base
    } else {
        ((base as f32 * mult.max(1.0)) as u32).max(base)
    }
}

impl Inventory {
    pub fn new(slots: usize) -> Inventory {
        Inventory {
            slots: vec![None; slots],
        }
    }

    pub fn resize(&mut self, slots: usize) -> Vec<Stack> {
        let mut spilled = Vec::new();
        while self.slots.len() > slots {
            if let Some(s) = self.slots.pop().flatten() {
                spilled.push(s);
            }
        }
        self.slots.resize(slots, None);
        spilled
    }

    pub fn count(&self, item: Item) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    pub fn free_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.is_none()).count()
    }

    pub fn stacks(&self) -> impl Iterator<Item = &Stack> {
        self.slots.iter().flatten()
    }

    /// How many of `item` still fit.
    pub fn room_for(&self, item: Item, mult: f32) -> u32 {
        let limit = stack_limit(item, mult);
        self.slots
            .iter()
            .map(|s| match s {
                None => limit,
                Some(s) if s.item == item => limit.saturating_sub(s.count),
                _ => 0,
            })
            .sum()
    }

    /// Adds items, topping up existing stacks first. Returns what didn't fit.
    pub fn add(&mut self, item: Item, mut count: u32, mult: f32) -> u32 {
        self.add_range(item, &mut count, mult, 0..self.slots.len());
        count
    }

    fn add_range(&mut self, item: Item, count: &mut u32, mult: f32, range: std::ops::Range<usize>) {
        let limit = stack_limit(item, mult);
        for pass in 0..2 {
            for i in range.clone() {
                if *count == 0 {
                    return;
                }
                match &mut self.slots[i] {
                    Some(s) if pass == 0 && s.item == item && s.count < limit => {
                        let n = (limit - s.count).min(*count);
                        s.count += n;
                        *count -= n;
                    }
                    slot @ None if pass == 1 => {
                        let n = limit.min(*count);
                        *slot = Some(Stack::new(item, n));
                        *count -= n;
                    }
                    _ => {}
                }
            }
        }
    }

    /// Removes `count` of `item` if that many are present. All or nothing.
    pub fn remove(&mut self, item: Item, count: u32) -> bool {
        if self.count(item) < count {
            return false;
        }
        self.take(item, count);
        true
    }

    /// Removes up to `count` of `item`, emptying non-favourite stacks first.
    /// Returns how many were taken.
    pub fn take(&mut self, item: Item, count: u32) -> u32 {
        let mut left = count;
        for fav_pass in [false, true] {
            for slot in self.slots.iter_mut().rev() {
                if left == 0 {
                    break;
                }
                if let Some(s) = slot
                    && s.item == item
                    && s.fav == fav_pass
                {
                    let n = s.count.min(left);
                    s.count -= n;
                    left -= n;
                    if s.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
        count - left
    }

    /// Moves up to `count` items from slot `from` to slot `to` within this
    /// inventory: merges into the same item, fills an empty slot, or swaps
    /// (only when moving the whole stack).
    pub fn move_within(&mut self, from: usize, to: usize, count: u32, mult: f32) {
        if from == to || from >= self.slots.len() || to >= self.slots.len() {
            return;
        }
        let (a, b) = if from < to {
            let (l, r) = self.slots.split_at_mut(to);
            (&mut l[from], &mut r[0])
        } else {
            let (l, r) = self.slots.split_at_mut(from);
            (&mut r[0], &mut l[to])
        };
        move_between(a, b, count, mult);
    }

    /// Groups stacks by category and item, merging them. Slots below `skip`
    /// (the hotbar) and favourites stay where they are.
    pub fn sort(&mut self, skip: usize, mult: f32) {
        let mut loose: Vec<Stack> = Vec::new();
        for slot in self.slots.iter_mut().skip(skip) {
            if slot.is_some_and(|s| !s.fav) {
                loose.push(slot.take().unwrap());
            }
        }
        loose.sort_by_key(|s| (category_order(s.item.category()), s.item));
        let mut merged: Vec<Stack> = Vec::new();
        for s in loose {
            let limit = stack_limit(s.item, mult);
            let mut count = s.count;
            if let Some(last) = merged.last_mut()
                && last.item == s.item
                && last.count < limit
            {
                let n = (limit - last.count).min(count);
                last.count += n;
                count -= n;
            }
            while count > 0 {
                let n = count.min(limit);
                merged.push(Stack::new(s.item, n));
                count -= n;
            }
        }
        let mut it = merged.into_iter();
        for slot in self.slots.iter_mut().skip(skip) {
            if slot.is_none() {
                *slot = it.next();
            }
        }
    }

    /// Moves every non-favourite stack from slots `skip..` into `dst`.
    /// With `only_matching`, moves only items `dst` already holds (quick
    /// stack). Returns how many items moved.
    pub fn deposit_into(&mut self, dst: &mut Inventory, skip: usize, only_matching: bool, mult: f32) -> u32 {
        let mut moved = 0;
        for slot in self.slots.iter_mut().skip(skip) {
            let Some(s) = slot else { continue };
            if s.fav || (only_matching && dst.count(s.item) == 0) {
                continue;
            }
            let left = dst.add(s.item, s.count, mult);
            moved += s.count - left;
            if left == 0 {
                *slot = None;
            } else {
                s.count = left;
            }
        }
        moved
    }

    /// Moves the stack in `slot` into `dst` wherever it fits (shift-click).
    pub fn quick_move(&mut self, slot: usize, dst: &mut Inventory, mult: f32) {
        self.quick_move_range(slot, dst, mult, 0..dst.slots.len());
    }

    /// Like [`Inventory::quick_move`], limited to a range of `dst` slots.
    pub fn quick_move_range(
        &mut self,
        slot: usize,
        dst: &mut Inventory,
        mult: f32,
        range: std::ops::Range<usize>,
    ) {
        let Some(Some(s)) = self.slots.get_mut(slot) else {
            return;
        };
        let mut count = s.count;
        let item = s.item;
        dst.add_range(
            item,
            &mut count,
            mult,
            range.start.min(dst.slots.len())..range.end.min(dst.slots.len()),
        );
        if count == 0 {
            self.slots[slot] = None;
        } else {
            s.count = count;
        }
    }
}

fn category_order(c: Category) -> u8 {
    match c {
        Category::Tool => 0,
        Category::Building => 1,
        Category::Resource => 2,
        Category::Seed => 3,
        Category::Crop => 4,
        Category::Food => 5,
        Category::Animal => 6,
    }
}

/// Moves up to `count` from slot `a` to slot `b` (which may be in different
/// inventories): merge, fill, or swap when the whole stack moves onto a
/// different item.
pub fn move_between(a: &mut Option<Stack>, b: &mut Option<Stack>, count: u32, mult: f32) {
    let Some(src) = a else { return };
    let count = count.min(src.count);
    if count == 0 {
        return;
    }
    match b {
        Some(dst) if dst.item == src.item => {
            let limit = stack_limit(dst.item, mult);
            let n = limit.saturating_sub(dst.count).min(count);
            dst.count += n;
            src.count -= n;
            if src.count == 0 {
                *a = None;
            }
        }
        Some(_) => {
            if count == src.count {
                std::mem::swap(a, b);
            }
        }
        None => {
            if count == src.count {
                *b = a.take();
            } else {
                src.count -= count;
                *b = Some(Stack::new(src.item, count));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tops_up_then_fills_and_reports_overflow() {
        let mut inv = Inventory::new(2);
        assert_eq!(inv.add(Item::Meal, 50, 1.0), 0);
        assert_eq!(inv.add(Item::Meal, 60, 1.0), 0);
        assert_eq!(inv.slots[0].unwrap().count, 99);
        assert_eq!(inv.slots[1].unwrap().count, 11);
        assert_eq!(inv.add(Item::Egg, 5, 1.0), 5, "no free slot for a new item");
        assert_eq!(inv.add(Item::Meal, 100, 1.0), 12);
        assert_eq!(inv.count(Item::Meal), 198);
        assert_eq!(inv.room_for(Item::Meal, 1.0), 0);
        // The stack multiplier raises the limit.
        assert_eq!(inv.add(Item::Meal, 100, 2.0), 0);
    }

    #[test]
    fn remove_is_all_or_nothing_and_spares_favourites() {
        let mut inv = Inventory::new(3);
        inv.add(Item::Iron, 10, 1.0);
        inv.slots[1] = Some(Stack {
            item: Item::Iron,
            count: 5,
            fav: true,
        });
        assert!(!inv.remove(Item::Iron, 16));
        assert_eq!(inv.count(Item::Iron), 15);
        assert!(inv.remove(Item::Iron, 8));
        assert_eq!(
            inv.slots[1].unwrap().count,
            5,
            "favourite untouched while others remain"
        );
        assert!(inv.remove(Item::Iron, 6));
        assert_eq!(inv.count(Item::Iron), 1);
    }

    #[test]
    fn moving_merges_splits_and_swaps() {
        let mut inv = Inventory::new(4);
        inv.slots[0] = Some(Stack::new(Item::Iron, 10));
        inv.slots[1] = Some(Stack::new(Item::Copper, 4));
        // Split half into an empty slot.
        inv.move_within(0, 2, 5, 1.0);
        assert_eq!(inv.slots[0].unwrap().count, 5);
        assert_eq!(inv.slots[2].unwrap().count, 5);
        // Merge back.
        inv.move_within(2, 0, 5, 1.0);
        assert_eq!(inv.slots[0].unwrap().count, 10);
        assert!(inv.slots[2].is_none());
        // Whole stack onto a different item swaps.
        inv.move_within(0, 1, 10, 1.0);
        assert_eq!(inv.slots[0].unwrap().item, Item::Copper);
        assert_eq!(inv.slots[1].unwrap().item, Item::Iron);
        // A partial move onto a different item does nothing.
        inv.move_within(1, 0, 3, 1.0);
        assert_eq!(inv.slots[1].unwrap().count, 10);
    }

    #[test]
    fn sort_groups_and_merges_but_keeps_hotbar_and_favourites() {
        let mut inv = Inventory::new(8);
        inv.slots[0] = Some(Stack::new(Item::Stone, 1));
        inv.slots[2] = Some(Stack::new(Item::Meal, 3));
        inv.slots[3] = Some(Stack::new(Item::Iron, 2));
        inv.slots[5] = Some(Stack {
            item: Item::Gold,
            count: 1,
            fav: true,
        });
        inv.slots[6] = Some(Stack::new(Item::Iron, 4));
        inv.slots[7] = Some(Stack::new(Item::WateringCan, 1));
        inv.sort(2, 1.0);
        assert_eq!(inv.slots[0].unwrap().item, Item::Stone, "hotbar untouched");
        assert_eq!(inv.slots[2].unwrap().item, Item::WateringCan, "tools first");
        assert_eq!(inv.slots[3].unwrap(), Stack::new(Item::Iron, 6), "stacks merged");
        assert_eq!(inv.slots[4].unwrap().item, Item::Meal);
        assert!(inv.slots[5].unwrap().fav, "favourite stays put");
        assert!(inv.slots[6].is_none() && inv.slots[7].is_none());
    }

    #[test]
    fn quick_stack_and_deposit() {
        let mut bag = Inventory::new(6);
        bag.add(Item::Iron, 30, 1.0);
        bag.add(Item::Copper, 12, 1.0);
        bag.add(Item::Gold, 3, 1.0);
        bag.slots[2].as_mut().unwrap().fav = true;
        let mut chest = Inventory::new(3);
        chest.add(Item::Iron, 5, 1.0);

        // Quick stack: only iron (already in the chest) moves.
        assert_eq!(bag.deposit_into(&mut chest, 0, true, 1.0), 30);
        assert_eq!(chest.count(Item::Iron), 35);
        assert_eq!(bag.count(Item::Copper), 12);

        // Deposit all: copper moves, favourite gold stays.
        assert_eq!(bag.deposit_into(&mut chest, 0, false, 1.0), 12);
        assert_eq!(bag.count(Item::Gold), 3);
        assert_eq!(chest.count(Item::Copper), 12);

        // Shift-click the gold: fits in the last free slot.
        bag.quick_move(2, &mut chest, 1.0);
        assert!(bag.is_empty());
        assert_eq!(chest.free_slots(), 0);
    }

    #[test]
    fn moves_between_inventories() {
        let mut a = Some(Stack::new(Item::Wood, 8));
        let mut b = None;
        move_between(&mut a, &mut b, 3, 1.0);
        assert_eq!(a.unwrap().count, 5);
        assert_eq!(b.unwrap().count, 3);
        let mut tool = Some(Stack::new(Item::WateringCan, 1));
        let mut other = Some(Stack::new(Item::WateringCan, 1));
        move_between(&mut tool, &mut other, 1, 5.0);
        assert!(tool.is_some(), "tools never stack");
    }
}
