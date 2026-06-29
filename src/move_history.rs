#[derive(Default, Clone)]
pub struct MoveHistory<T: Eq + Copy> {
    last_move: Option<T>,
    last_last_move: Option<T>,
}

impl<T: Eq + Copy> MoveHistory<T> {
    pub fn new() -> Self {
        Self {
            last_move: None,
            last_last_move: None,
        }
    }
    pub fn add(&mut self, t: T) {
        self.last_last_move = self.last_move;
        self.last_move = Some(t);
    }
    pub fn last(&self, t: T) -> bool {
        self.last_move == Some(t)
    }
    pub fn last_last(&self, t: T) -> bool {
        self.last_last_move == Some(t)
    }
    pub fn last_two(&self, t: T) -> bool {
        self.last_move == Some(t) && self.last_last_move == Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_history() {
        let mut mh = MoveHistory::<i32>::default();

        assert!(!mh.last(0));
        assert!(!mh.last_last(0));
        assert!(!mh.last_two(0));
        assert!(!mh.last(1));
        assert!(!mh.last_last(1));
        assert!(!mh.last_two(1));

        mh.add(0);

        assert!(mh.last(0));
        assert!(!mh.last_last(0));
        assert!(!mh.last_two(0));
        assert!(!mh.last(1));
        assert!(!mh.last_last(1));
        assert!(!mh.last_two(1));

        mh.add(0);

        assert!(mh.last(0));
        assert!(mh.last_last(0));
        assert!(mh.last_two(0));
        assert!(!mh.last(1));
        assert!(!mh.last_last(1));
        assert!(!mh.last_two(1));

        mh.add(1);

        assert!(!mh.last(0));
        assert!(mh.last_last(0));
        assert!(!mh.last_two(0));
        assert!(mh.last(1));
        assert!(!mh.last_last(1));
        assert!(!mh.last_two(1));

        mh.add(1);

        assert!(!mh.last(0));
        assert!(!mh.last_last(0));
        assert!(!mh.last_two(0));
        assert!(mh.last(1));
        assert!(mh.last_last(1));
        assert!(mh.last_two(1));
    }
}
