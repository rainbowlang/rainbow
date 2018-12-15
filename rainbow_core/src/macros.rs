macro_rules! dbg {
  ($fmt:expr $(, $thing:expr)*) => {
    if cfg!(test) {
      println!($fmt $(, $thing)*);
    }
  };
}
