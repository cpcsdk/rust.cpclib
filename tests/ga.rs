#[cfg(test)]
mod tests {
    use cpclib::ga::*;

    #[test]
    fn test_empty() {
        let mut p = Palette::empty();
        assert_eq!(p.pens().len(), 0);
        assert!(!p.contains_border());
    }

    #[test]
    fn test_default() {
        let mut p = Palette::default();
        assert_eq!(p.pens().len(), 16);
        assert_eq!(p.pens_with_border().len(), 17);
        assert!(p.contains_border());
        assert_eq!(p.inks().len(), 16);
        assert_eq!(p.inks_with_border().len(), 17);
    }

    #[test]
    fn test_new() {
        let mut p = Palette::new();
        assert_eq!(p.pens().len(), 16);
        assert_eq!(p.pens_with_border().len(), 17);
        assert!(p.contains_border());
        assert_eq!(p.inks().len(), 16);
        assert_eq!(p.inks_with_border().len(), 17);
    }
}
