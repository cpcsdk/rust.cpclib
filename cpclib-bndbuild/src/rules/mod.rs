pub mod graph;
pub mod rule;
pub mod rules;

pub use graph::*;
pub use rule::*;
pub use rules::*;

#[cfg(test)]
mod test {
    use crate::expand_glob;
    use crate::rules::{Rule, Rules};
    use crate::task::Task;

    #[test]
    fn test_deserialize_rule1() {
        let yaml = "targets: samourai.sna samourai.sym
dependencies: samourai.asm
commands:
  - basm samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym";
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rule.targets().len(), 2);
        assert_eq!(rule.dependencies().len(), 1);
        assert_eq!(
            *rule.command(0),
            Task::new_basm(
                "samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym"
            )
        );
    }

    #[test]
    fn test_deserialize_rule2() {
        let yaml = "targets: samourai.sna samourai.sym
dependencies: samourai.asm
commands: basm samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym";
        let rule: Rule = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rule.targets().len(), 2);
        assert_eq!(rule.dependencies().len(), 1);
        assert_eq!(
            *rule.command(0),
            Task::new_basm(
                "samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym"
            )
        );
    }

    #[test]
    fn test_deserialize_rules1() {
        let yaml = "- targets: samourai.sna samourai.sym
  dependencies: samourai.asm
  commands:
   - basm samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym
- targets: samourai.sna samourai.sym
  dependencies: samourai.asm
  commands:
    - basm samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym";
        let rules: Rules = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rules.rules().len(), 2);
    }

    #[test]
    fn test_glob_path() {
        let fname = "samourai.{lst,sym}";
        let result = expand_glob(fname);
        eprintln!("{:?}", result);
        assert_eq!(result.len(), 2);

        let fname = "samourai.{lst,sym";
        let result = expand_glob(fname);
        eprintln!("{:?}", result);
        assert_eq!(result.len(), 1);
    }
}
