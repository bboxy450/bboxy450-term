mkdir -p target && rustc --edition 2021 --test src/lib.rs -o target/atlas-test 2>&1 && ./target/atlas-test
