use steelcal_core::{
    compute_coil, compute_costs, compute_scrap, CoilInputs, CostInputs, PriceMode,
};

#[test]
fn compute_coil_requires_width_when_weight_is_present() {
    let err = compute_coil(&CoilInputs {
        coil_width_in: 0.0,
        coil_thickness_in: 0.05,
        coil_id_in: 20.0,
        coil_weight_lb: 1000.0,
        density_lb_ft3: 490.0,
    })
    .unwrap_err();

    assert!(err.to_string().contains("Coil width required"));
}

#[test]
fn compute_coil_rejects_negative_weight() {
    let err = compute_coil(&CoilInputs {
        coil_width_in: 48.0,
        coil_thickness_in: 0.05,
        coil_id_in: 20.0,
        coil_weight_lb: -1000.0,
        density_lb_ft3: 490.0,
    })
    .unwrap_err();

    assert!(err.to_string().contains("Coil weight"));
}

#[test]
fn compute_coil_rejects_negative_inner_diameter() {
    let err = compute_coil(&CoilInputs {
        coil_width_in: 48.0,
        coil_thickness_in: 0.05,
        coil_id_in: -20.0,
        coil_weight_lb: 1000.0,
        density_lb_ft3: 490.0,
    })
    .unwrap_err();

    assert!(err.to_string().contains("Coil ID"));
}

#[test]
fn compute_scrap_rejects_non_positive_ending_weight() {
    let err = compute_scrap(1000.0, 0.0, 0.35, 0.05).unwrap_err();
    assert!(err.to_string().contains("ending weight"));
}

#[test]
fn compute_costs_rejects_negative_setup_fee() {
    let err = compute_costs(
        &CostInputs {
            mode: PriceMode::PerLb,
            price_value: 0.5,
            markup_pct: 0.0,
            tax_pct: 0.0,
            setup_fee: -10.0,
            minimum_order: 0.0,
        },
        1,
        80.0,
        32.0,
    )
    .unwrap_err();

    assert!(err.to_string().contains("Setup fee"));
}
