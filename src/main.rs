mod patricia;

use patricia::PatriciaTrie;

fn main() {
    let mut trie = PatriciaTrie::new();
    for (i, word) in ["romane", "romanus", "romulus", "rubens", "ruber", "rubicon"]
        .iter()
        .enumerate()
    {
        trie.insert(word.as_bytes(), i);
    }

    println!("{} keys", trie.len());
    for probe in ["romane", "rubicon", "roman"] {
        println!("{probe:>8} -> {:?}", trie.get(probe.as_bytes()));
    }
}
