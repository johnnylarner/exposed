fn main() {
    for (left, right) in [
        ("andrew lichnowski", "andy lichnowski"),
        ("david luxton", "david lyon"),
        ("伊藤", "伊東"),
    ] {
        println!(
            "{}",
            serde_json::json!({"left":left,"right":right,
            "distance_at_most_2":weldrs::string_distance::levenshtein_within(left,right,2),
            "simd":cfg!(feature="splink-simd")})
        );
    }
}
