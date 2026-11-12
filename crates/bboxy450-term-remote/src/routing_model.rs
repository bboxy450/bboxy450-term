// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Two sessions on independent links. Socket/process tests cover adoption and
//! framing; this finite projection checks event recipients and MCP drop isolation.
macro_rules! routing {
    ($name:ident, $global:expr, $cleanup_all:expr) => {
        bbt_yarn::protocol! {
            pub mod $name {
                domain Connection { A, B }
                state {
                    a_live: bool = false;
                    b_live: bool = false;
                    latest: Option<Connection> = None;
                    b_mcp: bool = false;
                    misrouted: bool = false;
                    lost_other: bool = false;
                }
                events { ConnectA; ConnectB; OpenB; EventA; DropA; DropB; }
                on ConnectA {
                    case connect when true => {
                        set a_live = true;
                        set latest = Some(Connection::A);
                        preserve b_live, b_mcp, misrouted, lost_other;
                    }
                }
                on ConnectB {
                    case connect when true => {
                        set b_live = true;
                        set latest = Some(Connection::B);
                        preserve a_live, b_mcp, misrouted, lost_other;
                    }
                }
                on OpenB {
                    case live when b_live => {
                        set b_mcp = true;
                        preserve a_live, b_live, latest, misrouted, lost_other;
                    }
                    case down when !b_live => {
                        preserve a_live, b_live, latest, b_mcp, misrouted, lost_other;
                        ignore NoLink;
                    }
                }
                on EventA {
                    case live when a_live => {
                        set misrouted = misrouted || ($global && latest != Some(Connection::A));
                        preserve a_live, b_live, latest, b_mcp, lost_other;
                    }
                    case down when !a_live => {
                        preserve a_live, b_live, latest, b_mcp, misrouted, lost_other;
                        ignore LoggedForResume;
                    }
                }
                on DropA {
                    case close when true => {
                        set a_live = false;
                        set b_mcp = b_mcp && !$cleanup_all;
                        set lost_other = lost_other || ($cleanup_all && b_mcp);
                        preserve b_live, latest, misrouted;
                    }
                }
                on DropB {
                    case close when true => {
                        set b_live = false;
                        set b_mcp = false;
                        preserve a_live, latest, misrouted, lost_other;
                    }
                }
                #[invariant(id = "REMOTE-ROUTING-001")]
                fn event_goes_to_its_session() { !misrouted }
                #[invariant(id = "REMOTE-ROUTING-002")]
                fn disconnect_keeps_other_channels() { !lost_other }
            }
        }
    };
}
routing!(RemoteRouting, false, false);
routing!(BrokenRemoteRoutingOutput, true, false);
routing!(BrokenRemoteRoutingCleanup, false, true);
