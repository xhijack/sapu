fn main() {
    for a in sapu_core::apps::list_apps().into_iter().take(60) {
        println!("{:>12}  {}  {:?}", a.bytes, a.name, a.support);
    }
}
