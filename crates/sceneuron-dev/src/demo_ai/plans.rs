// Copyright (c) 2026 Alexander Picon, Chaski AI (https://chaski.ai). All rights reserved.
// Proprietary software; see LICENSE.

//! Simulated edit plans: keywords in the streamer's words pick ready-made features and card packs,
//! and anything else, or a pack already added along with an element or color, restyles one
//! element. Real plans come from a planner model; these only make the editor's assistant
//! demonstrable without one.

use sceneuron_core::{EditPlan, Project};
use sceneuron_server::{PlanMode, PlanRequest};

use super::features::{Feature, Planner};
use super::packs::{add_pack, in_project, packs_for};
use super::recipes::{celebrate, find_color, holding, pet, title_for, wave};
use super::restyle::restyle;
use super::scenes::mentions;

/// Things a streamer might hold up, named in the request; the first match is used.
const THINGS: [&str; 6] = ["keyboard", "miniature", "mug", "book", "sneaker", "plushie"];

const ANIMALS: [&str; 4] = ["cat", "dog", "kitten", "puppy"];

/// The plan for `request` against `project`; its operations apply cleanly to that project.
pub(crate) fn propose(project: &Project, request: &PlanRequest) -> EditPlan {
    let words = request.request.to_lowercase();
    let setup = request.mode == PlanMode::Setup;
    let mut features = features_for(&words);
    let packs = packs_for(&words, &project.canvas);
    if features.is_empty() && packs.is_empty() && setup {
        features = vec![holding("product"), wave()];
    }
    // Packs the project already has add nothing, so "Make the poker card blue" is a restyle.
    let adds_pack = packs.iter().any(|pack| !in_project(project, pack));
    if features.is_empty() && (packs.is_empty() || (!adds_pack && names_a_look(project, &words))) {
        return restyle(project, &request.request, &words);
    }
    let color = find_color(&words);
    let mut planner = Planner::new(project);
    let mut parts: Vec<String> = features
        .into_iter()
        .map(|feature| planner.add(feature, color, setup))
        .collect();
    parts.extend(packs.into_iter().map(|pack| add_pack(&mut planner, pack)));
    if let Some(title) = title_for(&words).filter(|_| setup)
        && planner.retitle(title)
    {
        parts.push(format!("title your stream \u{201c}{title}\u{201d}"));
    }
    let summary = if setup {
        format!("Simulated setup for your stream: {}.", join(&parts))
    } else {
        format!("Simulated plan: {}.", join(&parts))
    };
    planner.finish(summary)
}

fn features_for(words: &str) -> Vec<Feature> {
    let has = |keywords: &[&str]| keywords.iter().any(|keyword| mentions(words, keyword));
    let mut features = Vec::new();
    let thing = THINGS.iter().find(|thing| mentions(words, thing));
    if thing.is_some() || has(&["hold up", "holding", "product", "show its name", "showing", "review"]) {
        features.push(holding(thing.copied().unwrap_or("product")));
    }
    let animal = ANIMALS.iter().find(|animal| mentions(words, animal));
    if animal.is_some() || has(&["pet"]) {
        features.push(pet(animal.copied().unwrap_or("pet")));
    }
    if has(&["wave", "waving", "hello", "greet"]) {
        features.push(wave());
    }
    if has(&["arms up", "arms", "celebrat", "hype", "cheer"]) {
        features.push(celebrate());
    }
    features
}

/// Whether the request names one of the project's elements or a color.
fn names_a_look(project: &Project, words: &str) -> bool {
    let named = |name: &str| words.contains(&name.to_lowercase());
    find_color(words).is_some() || project.elements.iter().any(|element| named(&element.name))
}

/// "a", "a and b", or "a, b, and c".
fn join(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use sceneuron_core::{Canvas, EditOp, Project, default_project, pack_for_canvas};

    use super::*;

    fn ask(project: &Project, words: &str, mode: PlanMode) -> (EditPlan, Project) {
        let request = PlanRequest {
            request: words.to_owned(),
            mode,
            use_camera: false,
        };
        let plan = propose(project, &request);
        let applied = plan.apply(project).unwrap_or_else(|error| panic!("{words}: {error}"));
        (plan, applied)
    }

    fn ops(plan: &EditPlan) -> Vec<String> {
        let json = serde_json::to_value(&plan.operations).unwrap();
        json.as_array()
            .unwrap()
            .iter()
            .map(|op| op["op"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn waving_adds_a_condition_a_banner_and_two_rules() {
        let (plan, applied) = ask(&default_project(), "Say hi when I wave", PlanMode::Edit);
        assert_eq!(ops(&plan), ["add_condition", "add_element", "add_rule", "add_rule"]);
        assert!(plan.summary.starts_with("Simulated plan:"));
        assert!(applied.condition("waving").is_some());
        assert!(
            applied
                .element("wave-hello")
                .is_some_and(|element| !element.props.visible)
        );
        let (again, _) = ask(&applied, "wave", PlanMode::Edit);
        assert!(
            again.operations.is_empty(),
            "asking twice changes nothing: {:?}",
            again.operations
        );
    }

    #[test]
    fn holding_up_a_product_reuses_the_existing_lower_third_and_recolors_it() {
        let request = "When I hold up a product, show its name in a blue lower third";
        let (plan, applied) = ask(&default_project(), request, PlanMode::Edit);
        assert_eq!(ops(&plan), ["update_element"]);
        assert_eq!(applied.element("now-showing").unwrap().props.background, "#3b82f6ee");
        let empty = Project {
            elements: Vec::new(),
            conditions: Vec::new(),
            rules: Vec::new(),
            ..default_project()
        };
        let (fresh, applied) = ask(&empty, request, PlanMode::Edit);
        assert_eq!(ops(&fresh), ["add_condition", "add_element", "add_rule", "add_rule"]);
        let label = applied.element("product-name").unwrap();
        assert_eq!(label.props.text, "Now showing: {{showing-product}}");
    }

    #[test]
    fn setup_tailors_existing_conditions_and_adds_missing_features() {
        let description = "I review keyboards and my cat jumps on the desk, and I wave at new followers";
        let (plan, applied) = ask(&default_project(), description, PlanMode::Setup);
        assert!(plan.summary.starts_with("Simulated setup for your stream:"));
        let kinds = ops(&plan);
        assert!(kinds.contains(&"update_condition".to_owned()), "{kinds:?}");
        assert!(kinds.contains(&"add_condition".to_owned()), "{kinds:?}");
        assert!(applied.condition("showing-item").unwrap().prompt.contains("keyboard"));
        assert!(applied.condition("pet-in-view").unwrap().prompt.contains("cat"));
        assert!(applied.condition("waving").is_some());
        assert_eq!(
            applied.element("name-label").unwrap().props.text,
            "Keyboard reviews, live"
        );
        assert!(plan.summary.contains("title your stream"), "{}", plan.summary);
        let (edit, _) = ask(&default_project(), description, PlanMode::Edit);
        assert!(!edit.summary.contains("title"), "only a setup retitles the stream");
        let (vague, applied) = ask(&Project::default(), "just chatting", PlanMode::Setup);
        assert!(!vague.operations.is_empty());
        assert_eq!(applied.conditions.len(), 2);
    }

    #[test]
    fn meetings_sprint_planning_and_classes_add_the_matching_pack() {
        let base = default_project();
        for (request, pack) in [
            ("Give me signal cards for my Zoom meetings", "meeting"),
            ("I run sprint planning on Zoom", "poker"),
            ("I teach a class online", "classroom"),
        ] {
            let (plan, applied) = ask(&base, request, PlanMode::Setup);
            assert_eq!(ops(&plan), ["add_element", "add_deck"], "{request}");
            let expected = base.add_pack(&pack_for_canvas(pack, &base.canvas).unwrap()).unwrap();
            assert_eq!(applied, expected, "{request}");
            assert!(plan.summary.contains("deck of"), "{}", plan.summary);
            let (again, _) = ask(&applied, request, PlanMode::Edit);
            assert!(again.operations.is_empty(), "asking twice adds nothing: {request}");
            assert!(again.summary.contains("already have"), "{}", again.summary);
        }
    }

    #[test]
    fn an_element_or_color_of_a_pack_already_added_is_restyled() {
        let base = default_project();
        let project = base.add_pack(&pack_for_canvas("poker", &base.canvas).unwrap()).unwrap();
        let (plan, applied) = ask(&project, "Make the poker card blue", PlanMode::Edit);
        assert!(matches!(&plan.operations[..], [EditOp::UpdateElement { id, .. }] if id == "poker-card"));
        assert_eq!(applied.element("poker-card").unwrap().props.background, "#3b82f6ee");
        assert!(plan.summary.contains("restyle Poker card"), "{}", plan.summary);
        let (kept, _) = ask(&project, "Planning poker please", PlanMode::Edit);
        assert!(kept.operations.is_empty() && kept.summary.contains("already have"));
    }

    #[test]
    fn packs_fit_the_canvas_and_take_a_free_slot_id() {
        let mut project = Project {
            canvas: Canvas {
                width: 1280,
                height: 720,
            },
            ..default_project()
        };
        let mut taken = project.elements[0].clone();
        taken.id = "poker-card".to_owned();
        project.elements.push(taken);
        let (plan, applied) = ask(&project, "Planning poker and a cat alert", PlanMode::Edit);
        assert!(ops(&plan).ends_with(&["add_element".to_owned(), "add_deck".to_owned()]));
        let deck = applied.deck("poker").unwrap();
        assert_eq!(deck.element, "poker-card-2");
        let slot = applied.element("poker-card-2").unwrap();
        assert_eq!((slot.props.x, slot.props.width), (1040.0, 200.0));
        let summary = &plan.summary;
        assert!(
            summary.contains("announce your cat") && summary.contains("Planning poker deck"),
            "{summary}"
        );
    }

    #[test]
    fn anything_else_restyles_an_element() {
        let (plan, _) = ask(&default_project(), "make it fancier", PlanMode::Edit);
        assert_eq!(ops(&plan), ["update_element"]);
    }

    #[test]
    fn lists_read_naturally() {
        let parts = |items: &[&str]| items.iter().map(|item| (*item).to_owned()).collect::<Vec<_>>();
        assert_eq!(join(&parts(&["a", "b", "c"])), "a, b, and c");
        assert_eq!(join(&parts(&["a", "b"])), "a and b");
        assert_eq!(join(&parts(&["a"])), "a");
    }
}
