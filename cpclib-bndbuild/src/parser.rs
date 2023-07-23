#[cfg(test)]
mod test {
    #[test]
    fn build_rule() {
        let _content = "samourai.sna samourai.sym: samourai.asm\n\tbasm samourai.asm  --progress --snapshot -o samourai.sna  -Idata  --sym samourai.sym";
    }
}
