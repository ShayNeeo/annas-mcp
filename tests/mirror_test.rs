use annas_mcp::mirror::{Candidate, Heartbeat, MirrorResolver};

#[test]
fn test_normalize_base_url() {
    assert_eq!(
        MirrorResolver::normalize_base_url("https://annas-archive.li/"),
        "annas-archive.li"
    );
    assert_eq!(
        MirrorResolver::normalize_base_url("http://annas-archive.org"),
        "annas-archive.org"
    );
    assert_eq!(
        MirrorResolver::normalize_base_url("  annas-archive.se/  "),
        "annas-archive.se"
    );
}

#[test]
fn test_rank_candidates() {
    let c1 = Candidate {
        monitor_id: 1,
        base_url: "annas-archive.slow".to_string(),
        source_url: "https://annas-archive.slow/".to_string(),
        heartbeats: vec![
            Heartbeat { status: 1, ping: 300, time: None },
            Heartbeat { status: 1, ping: 350, time: None },
        ],
    };

    let c2 = Candidate {
        monitor_id: 2,
        base_url: "annas-archive.fast".to_string(),
        source_url: "https://annas-archive.fast/".to_string(),
        heartbeats: vec![
            Heartbeat { status: 1, ping: 45, time: None },
            Heartbeat { status: 1, ping: 50, time: None },
        ],
    };

    let c3 = Candidate {
        monitor_id: 3,
        base_url: "annas-archive.down".to_string(),
        source_url: "https://annas-archive.down/".to_string(),
        heartbeats: vec![
            Heartbeat { status: 0, ping: 0, time: None },
        ],
    };

    let ranked = MirrorResolver::rank_candidates(vec![c1, c2, c3]);
    // Down mirror should be filtered out, and fast mirror ranked #1
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].base_url, "annas-archive.fast");
    assert_eq!(ranked[1].base_url, "annas-archive.slow");
}
