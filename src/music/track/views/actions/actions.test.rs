use super::page_link;

#[test]
fn channel_links_work_from_a_channel_id_or_a_handle() {
    assert_eq!(
        page_link("channel", "UC_gpi1zIV_AfdZ0Bg1Vcxaw").as_deref(),
        Some("https://www.youtube.com/channel/UC_gpi1zIV_AfdZ0Bg1Vcxaw")
    );
    assert_eq!(
        page_link("channel", "youtube:@PepoTech").as_deref(),
        Some("https://www.youtube.com/@PepoTech")
    );
    assert_eq!(page_link("channel", ""), None);
}
