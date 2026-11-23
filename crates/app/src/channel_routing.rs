use gsh_core::workspace::{ChannelMember, ChannelReplyRule};

pub(crate) fn recipients<'a>(
    rule: ChannelReplyRule,
    members: &'a [ChannelMember],
    text: &str,
) -> Vec<&'a ChannelMember> {
    let mut selected = Vec::new();
    let mut select = |member: &'a ChannelMember| {
        if !selected
            .iter()
            .any(|current: &&ChannelMember| current.same_session(member))
        {
            selected.push(member);
        }
    };
    match rule {
        ChannelReplyRule::Manual => {}
        ChannelReplyRule::All => members.iter().for_each(select),
        ChannelReplyRule::Mention => {
            let text = text.to_lowercase();
            let labels: Vec<_> = members
                .iter()
                .map(|member| (member.label().to_lowercase(), member))
                .collect();
            for (index, _) in text.match_indices('@') {
                let (before, mention) = text.split_at(index);
                if before.chars().next_back().is_some_and(word_char) {
                    continue;
                }
                let value = &mention[1..];
                let best = labels
                    .iter()
                    .filter(|(label, _)| {
                        !label.is_empty()
                            && value.strip_prefix(label.as_str()).is_some_and(|rest| {
                                rest.chars().next().is_none_or(|ch| !word_char(ch))
                            })
                    })
                    .max_by_key(|(label, _)| label.len());
                if let Some((_, member)) = best {
                    select(member);
                }
            }
        }
    }
    selected
}

fn word_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '-' | '@')
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_core::workspace::WorkspaceRef;

    fn member(label: &str, session_id: &str) -> ChannelMember {
        ChannelMember::Session {
            workspace: WorkspaceRef::System("terminal".into()),
            session_id: session_id.into(),
            label: label.into(),
        }
    }

    #[test]
    fn manual_and_all_resolve_without_consulting_mentions() {
        let members = [
            member("Ferro", "first"),
            member("Outro nome", "first"),
            member("Bulma", "second"),
        ];
        assert!(recipients(ChannelReplyRule::Manual, &members, "@Ferro").is_empty());
        assert_eq!(
            recipients(ChannelReplyRule::All, &members, ""),
            vec![&members[0], &members[2]]
        );
    }

    #[test]
    fn mention_resolves_the_longest_label_and_preserves_first_mention_order() {
        let members = [
            member("João", "first"),
            member("João Pedro", "second"),
            member("Ferro-Sol", "third"),
        ];
        assert_eq!(
            recipients(
                ChannelReplyRule::Mention,
                &members,
                "(@JOÃO PEDRO), veja isto, @Ferro-Sol. @João Pedro e @joão também."
            ),
            vec![&members[1], &members[2], &members[0]]
        );
        assert!(recipients(
            ChannelReplyRule::Mention,
            &members,
            "time@joão.test @@João @Joãozinho @João-Silva @João_Pedro"
        )
        .is_empty());
    }
}
