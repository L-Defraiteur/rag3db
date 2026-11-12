mod billing {
    pub struct Box {
        pub count: usize,
        label: String,
    }
    pub trait Policy {
        fn retry(&self);
    }
    impl Box {
        pub fn new() -> Self { Self { count: 0, label: String::new() } }
        pub fn size(&self) -> usize { self.count }
    }
    impl Policy for Box {
        fn retry(&self) {
            fn nested() -> usize { 1 }
            let _ = nested();
        }
    }
}

mod compact { pub struct Item {} }
