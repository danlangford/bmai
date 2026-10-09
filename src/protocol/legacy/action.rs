// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl Parser {
    pub(super) fn send_action<W: Write>(&mut self, output: &mut W) -> Result<(), ParseError> {
        match self.game.phase {
            Phase::Auxiliary => {
                let (choice, _) =
                    self.decide(|engine, game, context| engine.auxiliary(game, context));
                let original = choice
                    .choice
                    .map(|index| self.game.players[0].dice[index].original_index);
                self.last_action =
                    Some(crate::protocol::ProtocolAction::Auxiliary { die: original });
                if let Some(search) = choice.search {
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        search.score,
                        search.win_probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                writeln!(output, "aux {}", original.map_or(-1, |index| index as i32))
                    .map_err(io_error)
            }
            Phase::Fight => {
                let engine = self.player_engine(0);
                if self.report_sims > 0 && engine.montecarlo().is_none() {
                    return Err(ParseError("report_sims requires montecarlo search".into()));
                }
                if let Some(settings) = engine.montecarlo() {
                    let moves = self.game.generate_valid_attacks_in_cpp_order_for_search(
                        settings.fire_candidate_limit(),
                    );
                    writeln!(
                        output,
                        "l1 p0 Valid Moves {} Sims {}",
                        moves.len(),
                        settings.compute_number_sims(moves.len().max(1), 1)
                    )
                    .map_err(io_error)?;
                }
                let (choice, replay) =
                    self.decide(|engine, game, context| engine.attack(game, context));
                let action = choice.choice;
                // An exact endgame answer needs no resampled estimate.
                let exact = choice
                    .search
                    .as_ref()
                    .is_some_and(|search| search.simulations == 0);
                self.last_action = Some(protocol_attack(&self.game, &action)?);
                if let Some(search) = choice.search {
                    self.last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(
                            search.win_probability,
                        ),
                        simulations: search.simulations,
                        source: "move_selection",
                    });
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        search.score,
                        search.win_probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                if let (Some(settings), Some(replay), true) =
                    (engine.montecarlo(), replay, self.report_sims > 0 && !exact)
                {
                    let estimate = evaluate_selected_native_bmai_move(
                        &self.game,
                        &action,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        settings,
                        self.report_sims,
                    );
                    let win_probability = estimate.win_probability();
                    self.last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(win_probability),
                        simulations: estimate.simulations,
                        source: "selected_move_resample",
                    });
                    writeln!(
                        output,
                        "l1 p0 selected move report ({:.1}/{}, {:.1}% win)",
                        estimate.score,
                        estimate.simulations,
                        win_probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                send_attack(&self.game, &action, output)
            }
            Phase::Reserve => {
                let (reserve, _) =
                    self.decide(|engine, game, context| engine.reserve(game, context));
                self.last_action = Some(crate::protocol::ProtocolAction::Reserve {
                    die: reserve.map(|index| self.game.players[0].dice[index].original_index),
                });
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if let Some(index) = reserve {
                    writeln!(
                        output,
                        "reserve {}",
                        self.game.players[0].dice[index].original_index
                    )
                    .map_err(io_error)
                } else {
                    writeln!(output, "reserve -1").map_err(io_error)
                }
            }
            Phase::Preround => {
                let (action, _) =
                    self.decide(|engine, game, context| engine.swing(game, 0, context));
                self.last_action = Some(protocol_swing(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                send_set_swing(&self.game, &action, output)
            }
            Phase::Chance => {
                let (action, _) =
                    self.decide(|engine, game, context| engine.chance(game, 0, 1, context));
                self.last_action = Some(protocol_chance(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.reroll.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for index in action.reroll {
                        writeln!(
                            output,
                            "chance {}",
                            self.game.players[0].dice[index].original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            Phase::Focus => {
                let (action, _) =
                    self.decide(|engine, game, context| engine.focus(game, 0, 1, context));
                self.last_action = Some(protocol_focus(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.values.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for (index, value) in action.values {
                        writeln!(
                            output,
                            "focus {} {value}",
                            self.game.players[0].dice[index].original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            _ => Err(ParseError("GetAction(): Unrecognized phase".into())),
        }
    }

    /// Player 0's engine decides, drawing a native replay key only if it
    /// searches, and the key it drew is recorded for the session metadata.
    fn decide<T>(
        &mut self,
        decision: impl FnOnce(&dyn Engine, &Game, &mut DecisionContext<'_, '_>) -> T,
    ) -> (T, Option<crate::native::NativeReplayKey>) {
        let engine = self.player_engine(0);
        let mut sequence = NativeReplaySequence {
            algorithm: self.rng.algorithm(),
            root_seed: self.native_root_seed,
            workers: self.native_workers,
            decision_index: &mut self.native_decision_index,
        };
        let mut context = DecisionContext::new(&mut self.rng, Some(&mut sequence));
        let result = decision(engine.as_ref(), &self.game, &mut context);
        let replay = context.issued_replay();
        if let Some(replay) = replay {
            self.last_replay = Some(crate::protocol::ReplayMetadata {
                stream_partition: replay.stream_version.partition_id(),
                root_seed: replay.root_seed,
                decision_index: replay.decision_index,
            });
        }
        (result, replay)
    }

    pub(super) fn send_stats<W: Write>(&self, output: &mut W) -> Result<(), ParseError> {
        writeln!(
            output,
            "stats {}/{}-{}/{}/0.50",
            self.ai.max_ply, self.ai.min_sims, self.ai.max_sims, self.ai.max_branch
        )
        .map_err(io_error)
    }
}
