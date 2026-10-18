//! Shared mapping from visible Question Library order to the typed store order.

use question_model::LibraryObjectSearchSort;

pub(super) fn store_sort(
    sort: LibraryObjectSearchSort,
) -> learning_data_access::QuestionLibrarySearchSort {
    use learning_data_access::QuestionLibrarySearchSort as Store;
    match sort {
        LibraryObjectSearchSort::TitleAscending => Store::TitleAscending,
        LibraryObjectSearchSort::TitleDescending => Store::TitleDescending,
        LibraryObjectSearchSort::KindAscending => Store::KindAscending,
        LibraryObjectSearchSort::KindDescending => Store::KindDescending,
        LibraryObjectSearchSort::DisciplineAscending => Store::DisciplineAscending,
        LibraryObjectSearchSort::DisciplineDescending => Store::DisciplineDescending,
        LibraryObjectSearchSort::QuestionTypeAscending => Store::QuestionTypeAscending,
        LibraryObjectSearchSort::QuestionTypeDescending => Store::QuestionTypeDescending,
        LibraryObjectSearchSort::PublishedNewest => Store::PublishedNewest,
    }
}
