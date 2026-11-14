//! COCO category ids and labels, as used by the RF-DETR models.

/// `(category id, label)` pairs, sorted by id. Ids are not contiguous.
const LABELS: [(u16, &str); 80] = [
    (1, "person"),
    (2, "bicycle"),
    (3, "car"),
    (4, "motorcycle"),
    (5, "airplane"),
    (6, "bus"),
    (7, "train"),
    (8, "truck"),
    (9, "boat"),
    (10, "traffic light"),
    (11, "fire hydrant"),
    (13, "stop sign"),
    (14, "parking meter"),
    (15, "bench"),
    (16, "bird"),
    (17, "cat"),
    (18, "dog"),
    (19, "horse"),
    (20, "sheep"),
    (21, "cow"),
    (22, "elephant"),
    (23, "bear"),
    (24, "zebra"),
    (25, "giraffe"),
    (27, "backpack"),
    (28, "umbrella"),
    (31, "handbag"),
    (32, "tie"),
    (33, "suitcase"),
    (34, "frisbee"),
    (35, "skis"),
    (36, "snowboard"),
    (37, "sports ball"),
    (38, "kite"),
    (39, "baseball bat"),
    (40, "baseball glove"),
    (41, "skateboard"),
    (42, "surfboard"),
    (43, "tennis racket"),
    (44, "bottle"),
    (46, "wine glass"),
    (47, "cup"),
    (48, "fork"),
    (49, "knife"),
    (50, "spoon"),
    (51, "bowl"),
    (52, "banana"),
    (53, "apple"),
    (54, "sandwich"),
    (55, "orange"),
    (56, "broccoli"),
    (57, "carrot"),
    (58, "hot dog"),
    (59, "pizza"),
    (60, "donut"),
    (61, "cake"),
    (62, "chair"),
    (63, "couch"),
    (64, "potted plant"),
    (65, "bed"),
    (67, "dining table"),
    (70, "toilet"),
    (72, "tv"),
    (73, "laptop"),
    (74, "mouse"),
    (75, "remote"),
    (76, "keyboard"),
    (77, "cell phone"),
    (78, "microwave"),
    (79, "oven"),
    (80, "toaster"),
    (81, "sink"),
    (82, "refrigerator"),
    (84, "book"),
    (85, "clock"),
    (86, "vase"),
    (87, "scissors"),
    (88, "teddy bear"),
    (89, "hair drier"),
    (90, "toothbrush"),
];

/// The label for a COCO category id, if it is one of the 80 named classes.
#[must_use]
pub fn label(class_id: u16) -> Option<&'static str> {
    LABELS
        .binary_search_by_key(&class_id, |&(id, _)| id)
        .ok()
        .map(|i| LABELS[i].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_the_classes_used_by_the_golden_images() {
        for (id, expected) in [
            (1, "person"),
            (51, "bowl"),
            (54, "sandwich"),
            (63, "couch"),
            (86, "vase"),
        ] {
            assert_eq!(label(id), Some(expected));
        }
    }

    #[test]
    fn unnamed_ids_have_no_label() {
        for id in [0, 12, 26, 91, 500] {
            assert_eq!(label(id), None);
        }
    }

    #[test]
    fn table_is_sorted_and_unique_so_binary_search_is_valid() {
        assert!(LABELS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
}
