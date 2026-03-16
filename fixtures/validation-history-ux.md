# Desktop History & UX Polish — Validation Assertions

---

## Area: History Recording (HIST)

### VAL-HIST-001: Sheet calculation is recorded to SessionHistory
After a successful sheet calculation (width, length, qty, gauge/psf/thickness, pricing), a `HistoryEntry` with `entry_type == Sheet` is appended to the `SessionHistory`.
**Pass condition:** `session_history.get_entries()` contains a new entry with `entry_type == HistoryEntryType::Sheet` after each sheet calculation.
**Evidence:** Call calculate on the Sheet panel → inspect `SessionHistory::get_entries()` last element; verify `entry_type` is `Sheet`.

### VAL-HIST-002: Coil calculation is recorded to SessionHistory
After a successful coil calculation (width, thickness, ID, weight), a `HistoryEntry` with `entry_type == Coil` is appended to the `SessionHistory`.
**Pass condition:** `session_history.get_entries()` contains a new entry with `entry_type == HistoryEntryType::Coil` after each coil calculation.
**Evidence:** Call calculate on the Coil panel → inspect last entry; verify `entry_type` is `Coil`.

### VAL-HIST-003: Scrap calculation is recorded to SessionHistory
After a successful scrap/pickup calculation (actual weight, ending weight, base cost, processing cost), a `HistoryEntry` with `entry_type == Scrap` is appended to the `SessionHistory`.
**Pass condition:** `session_history.get_entries()` contains a new entry with `entry_type == HistoryEntryType::Scrap` after each scrap calculation.
**Evidence:** Call calculate on the Scrap panel → inspect last entry; verify `entry_type` is `Scrap`.

### VAL-HIST-004: Pricing-only calculation is recorded to SessionHistory
When pricing fields are populated alongside a sheet calculation (price mode, price value, markup %, tax %, setup fee, min order), a `HistoryEntry` with `entry_type == Pricing` is appended if pricing is computed independently, or the pricing inputs/outputs are captured within the Sheet entry.
**Pass condition:** Pricing data appears either as a separate `Pricing` entry or embedded in the `Sheet` entry's `inputs`/`outputs` JSON.
**Evidence:** Perform a sheet calculation with pricing → inspect entry's `inputs` for pricing fields (`price_mode`, `price_value`, `markup_pct`, `tax_pct`, `setup_fee`, `min_order`) and `outputs` for pricing results (`each_bt`, `each_at`, `total_bt`, `total_at`).

### VAL-HIST-005: Entry timestamp is accurate
Each `HistoryEntry` has a `timestamp` field set to the Unix epoch (seconds) at the moment the entry was created.
**Pass condition:** `entry.timestamp` is within ±2 seconds of `SystemTime::now()` at the time of the calculation.
**Evidence:** Record the wall-clock time before and after a calculation; verify `entry.timestamp` falls within that window.

### VAL-HIST-006: Entry inputs capture all user-provided values
Each `HistoryEntry.inputs` JSON object contains all fields the user entered for the corresponding calculation type:
- **Sheet:** width, length, qty, use_gauge, table_choice, gauge, use_psf, psf, thickness, price_mode, price_value, markup_pct, tax_pct, setup_fee, min_order
- **Coil:** coil_width, coil_thickness, coil_id, coil_weight
- **Scrap:** actual_weight, ending_weight, base_cost_per_lb, processing_cost_per_lb

**Pass condition:** The `inputs` JSON contains all expected keys with values matching what was entered.
**Evidence:** Enter known values → calculate → deserialize `entry.inputs` and compare each field.

### VAL-HIST-007: Entry outputs capture all computed results
Each `HistoryEntry.outputs` JSON object contains all calculated result values:
- **Sheet:** mass_each, mass_total, psf_result, each_bt, each_at, total_bt, total_at
- **Coil:** coil_od, coil_footage, coil_piw
- **Scrap:** scrap_lb, total_cost, price_per_lb, scrap_charge_per_lb, is_pickup

**Pass condition:** The `outputs` JSON contains all expected keys with correct calculated values.
**Evidence:** Perform a calculation with known inputs → verify outputs JSON matches the displayed results.

### VAL-HIST-008: Failed calculations are not recorded
If a calculation fails due to validation errors (e.g., missing required fields, non-numeric input), no entry is appended to `SessionHistory`.
**Pass condition:** `session_history.get_entries().len()` does not change after a failed calculation.
**Evidence:** Attempt a calculation with invalid input (e.g., empty width on sheet) → confirm history length unchanged.

### VAL-HIST-009: History entries preserve insertion order
Entries in `SessionHistory` are ordered oldest-first, matching the order calculations were performed.
**Pass condition:** After performing calculations A, B, C in order, `get_entries()[0]` corresponds to A, `[1]` to B, `[2]` to C.
**Evidence:** Perform three calculations of different types → verify ordering via timestamps and entry types.

---

## Area: History View Dialog (HIST)

### VAL-HIST-010: History dialog opens from Help menu
The Help menu contains a "View History" item. Clicking it opens the History dialog.
**Pass condition:** The History dialog window becomes visible after clicking Help > View History.
**Evidence:** Click Help > View History → dialog appears with title "Calculation History".

### VAL-HIST-011: History dialog opens via Ctrl+H shortcut
Pressing Ctrl+H opens the History dialog regardless of which panel has focus.
**Pass condition:** The History dialog window becomes visible after pressing Ctrl+H from any panel.
**Evidence:** Focus the Coil panel → press Ctrl+H → dialog opens.

### VAL-HIST-012: Empty history shows informational message
When no calculations have been performed and the user opens the history dialog, an informational message is shown instead of an empty list.
**Pass condition:** A message like "No history available" is displayed (either in-dialog or as a notification).
**Evidence:** Launch app → immediately press Ctrl+H → observe message.

### VAL-HIST-013: History list shows all session entries
The History dialog displays a scrollable list of all entries in the current session, each showing timestamp and calculation type.
**Pass condition:** The list contains exactly N items matching `session_history.get_entries().len()`, each labeled with its timestamp and type (e.g., "2025-08-23 12:30:00 - Sheet Calculation").
**Evidence:** Perform 5 calculations → open history → count list items; verify all 5 appear.

### VAL-HIST-014: Filter by type shows only matching entries
The History dialog provides a Type filter dropdown with options: All, Sheet, Coil, Scrap. Selecting a type filters the displayed list.
**Pass condition:** After selecting "Sheet", only entries with `entry_type == Sheet` appear. Selecting "All" shows all entries.
**Evidence:** Perform 2 sheet + 1 coil calculations → open history → select "Sheet" filter → only 2 entries visible → select "All" → 3 entries visible.

### VAL-HIST-015: Search filters entries by text match
The History dialog provides a search/filter text field. Typing a search term filters the displayed list to entries whose display text contains the term (case-insensitive).
**Pass condition:** Typing "coil" shows only coil entries; clearing the search field restores all entries.
**Evidence:** Perform mixed calculations → open history → type "coil" in search → only coil entries shown.

### VAL-HIST-016: Preview shows calculation details
Selecting an entry in the history list displays a detailed preview of that entry's inputs and outputs in a read-only text area.
**Pass condition:** The preview area shows the full summary text including all inputs and computed outputs for the selected entry.
**Evidence:** Click on a sheet entry → preview area shows width, length, qty, gauge, weight results, pricing, etc.

### VAL-HIST-017: Recall repopulates input fields for sheet entries
Clicking "Recall" with a sheet entry selected populates the Sheet panel fields with the entry's stored inputs: width, length, qty, gauge settings, pricing fields.
**Pass condition:** All sheet and pricing input fields match the recalled entry's stored values. The history dialog closes after recall.
**Evidence:** Perform a sheet calculation with known values → clear fields → open history → select the entry → click Recall → verify all fields restored.

### VAL-HIST-018: Recall repopulates input fields for coil entries
Clicking "Recall" with a coil entry selected populates the Coil panel fields with the entry's stored inputs: coil_width, coil_thickness, coil_id, coil_weight.
**Pass condition:** All coil input fields match the recalled entry's stored values. The history dialog closes after recall.
**Evidence:** Perform a coil calculation → clear → open history → select coil entry → Recall → verify coil fields restored.

### VAL-HIST-019: Recall repopulates input fields for scrap entries
Clicking "Recall" with a scrap entry selected populates the Scrap panel fields with the entry's stored inputs: actual_weight, ending_weight, base_cost_per_lb, processing_cost_per_lb.
**Pass condition:** All scrap input fields match the recalled entry's stored values. The history dialog closes after recall.
**Evidence:** Perform a scrap calculation → clear → open history → select scrap entry → Recall → verify scrap fields restored.

### VAL-HIST-020: Export saves history to text file
Clicking "Export" in the History dialog writes all currently displayed (filtered) entries to a text file in the stable `export_to_text` format.
**Pass condition:** A text file is created/overwritten containing the version header, separator, and one block per displayed entry matching the `export_to_text` format. A confirmation message is shown.
**Evidence:** Perform calculations → open history → click Export → read the output file → verify format matches `export_to_text` output with correct version header, timestamps, types, inputs, and outputs.

### VAL-HIST-021: Export respects active filter
When a type filter or search term is active, Export writes only the currently displayed (filtered) entries, not the entire session history.
**Pass condition:** With "Sheet" filter active, the exported file contains only sheet entries.
**Evidence:** Perform 2 sheet + 1 coil → open history → filter by "Sheet" → Export → file contains exactly 2 entries, both Sheet type.

### VAL-HIST-022: History dialog is modal
While the History dialog is open, the main application window does not accept input.
**Pass condition:** Clicking on the main window behind the dialog does not change focus or allow edits until the dialog is closed.
**Evidence:** Open history dialog → attempt to click/type in main window → no response.

---

## Area: UX Polish — Clear (UX)

### VAL-UX-001: Clear button on Sheet panel resets all sheet fields to defaults
Clicking the Clear button (or per-panel clear) resets all Sheet panel fields: width → empty, length → empty, qty → "1", psf → empty, table → config default, gauge → config default, thickness → empty, mass_each → empty, mass_total → empty, psf_result → empty.
**Pass condition:** Every sheet input and output field matches its default value after clear.
**Evidence:** Fill all sheet fields with non-default values → click Clear → verify each field.

### VAL-UX-002: Clear button on Coil panel resets all coil fields to defaults
Clicking the Clear button resets all Coil panel fields: coil_width → empty, coil_thickness → empty, coil_id → "20", coil_weight → empty, coil_od → empty, coil_footage → empty, coil_piw → empty.
**Pass condition:** Every coil input and output field matches its default value after clear.
**Evidence:** Fill all coil fields → click Clear → verify each field.

### VAL-UX-003: Clear button on Scrap panel resets all scrap fields to defaults
Clicking the Clear button resets all Scrap panel fields: base_cost_per_lb → "0.00", processing_cost_per_lb → "0.00", actual_weight → empty, ending_weight → empty, scrap_lb → empty, total_cost → empty, price_per_lb → empty, scrap_charge_per_lb → empty.
**Pass condition:** Every scrap input and output field matches its default value after clear.
**Evidence:** Fill all scrap fields → click Clear → verify each field.

### VAL-UX-004: Clear button resets pricing fields to defaults
Clicking Clear also resets all pricing fields: price_mode → "per lb", price_value → "0.00", markup_pct → "0", tax_pct → "0", setup_fee → "0.00", min_order → "0.00", each_bt → empty, each_at → empty, total_bt → empty, total_at → empty, min_note → empty.
**Pass condition:** Every pricing input and output field matches its default value.
**Evidence:** Fill all pricing fields → click Clear → verify each field.

### VAL-UX-005: Clear does not erase history
Clicking Clear resets input/output fields but does not remove any entries from `SessionHistory`.
**Pass condition:** `session_history.get_entries().len()` is unchanged after clear.
**Evidence:** Perform 3 calculations → click Clear → verify history still has 3 entries.

### VAL-UX-006: Clear returns focus to first input field
After clearing, keyboard focus returns to the first input field (table/material combo box on Sheet panel).
**Pass condition:** The table combo box (or first focusable input) has focus after clear.
**Evidence:** Click Clear → verify focused element is the material type combo.

---

## Area: UX Polish — Copy to Scrap (UX)

### VAL-UX-007: Copy to Actual copies sheet total weight to scrap actual weight field
Clicking "Copy to Actual" on the Scrap panel copies the current value of `mass_total` (Sheet panel total weight result) into the `actual_weight` scrap input field.
**Pass condition:** `actual_weight` field value equals the current `mass_total` display value.
**Evidence:** Calculate a sheet (mass_total = "1234.567") → click "Copy to Actual" → actual_weight field shows "1234.567".

### VAL-UX-008: Copy to Ending copies sheet total weight to scrap ending weight field
Clicking "Copy to Ending" on the Scrap panel copies the current value of `mass_total` (Sheet panel total weight result) into the `ending_weight` scrap input field.
**Pass condition:** `ending_weight` field value equals the current `mass_total` display value.
**Evidence:** Calculate a sheet (mass_total = "1234.567") → click "Copy to Ending" → ending_weight field shows "1234.567".

### VAL-UX-009: Copy to Actual with empty total weight copies empty string
If no sheet calculation has been performed (mass_total is empty), "Copy to Actual" sets the actual_weight field to empty string.
**Pass condition:** `actual_weight` field is empty after clicking "Copy to Actual" when mass_total is empty.
**Evidence:** Launch app (mass_total is blank) → click "Copy to Actual" → actual_weight field is blank.

### VAL-UX-010: Copy to Ending with empty total weight copies empty string
If no sheet calculation has been performed (mass_total is empty), "Copy to Ending" sets the ending_weight field to empty string.
**Pass condition:** `ending_weight` field is empty after clicking "Copy to Ending" when mass_total is empty.
**Evidence:** Launch app (mass_total is blank) → click "Copy to Ending" → ending_weight field is blank.

### VAL-UX-011: Copy buttons do not trigger a calculation
Clicking "Copy to Actual" or "Copy to Ending" only copies the value; it does not trigger a scrap calculation or add a history entry.
**Pass condition:** No new history entry is created, and scrap output fields remain unchanged.
**Evidence:** Calculate a sheet → click "Copy to Actual" → verify no scrap outputs populated and history length unchanged.

---

## Area: Keyboard Shortcuts (KEYS)

### VAL-KEYS-001: F1 opens Help dialog
Pressing F1 from any panel or field opens the Help/User Guide dialog.
**Pass condition:** The Help dialog window becomes visible with the user guide text.
**Evidence:** Focus a coil field → press F1 → help dialog appears.

### VAL-KEYS-002: Ctrl+H opens History dialog
Pressing Ctrl+H from any panel or field opens the History View dialog.
**Pass condition:** The History dialog becomes visible (or "No history" message if empty).
**Evidence:** Focus any field → press Ctrl+H → history dialog opens.

### VAL-KEYS-003: Ctrl+E opens Configuration editor
Pressing Ctrl+E from any panel or field opens the Configuration editor dialog.
**Pass condition:** The Configuration editor dialog becomes visible.
**Evidence:** Focus any field → press Ctrl+E → config editor opens.

### VAL-KEYS-004: Ctrl+Enter triggers Sheet calculation
Pressing Ctrl+Enter from any panel or field triggers the Sheet & Quote calculation (equivalent to clicking the "Calculate Sheet & Quote" button).
**Pass condition:** Sheet calculation executes; results appear in output fields (or validation error shown).
**Evidence:** Fill sheet inputs → focus a coil field → press Ctrl+Enter → sheet results populate.

### VAL-KEYS-005: Ctrl+Shift+C triggers Coil calculation
Pressing Ctrl+Shift+C from any panel or field triggers the Coil calculation.
**Pass condition:** Coil calculation executes; results appear in coil output fields (or validation error shown).
**Evidence:** Fill coil inputs → focus a sheet field → press Ctrl+Shift+C → coil results populate.

### VAL-KEYS-006: Escape closes any open dialog
Pressing Escape while any dialog (Help, History, Configuration, Summary) is open closes that dialog.
**Pass condition:** The topmost open dialog closes; focus returns to the main window.
**Evidence:** Open Help dialog → press Escape → dialog closes. Open History → press Escape → dialog closes.

### VAL-KEYS-007: Keyboard shortcuts work regardless of focus
All global shortcuts (F1, Ctrl+H, Ctrl+E, Ctrl+Enter, Ctrl+Shift+C, Escape) function correctly regardless of which panel or input field currently has focus.
**Pass condition:** Each shortcut produces its expected action when focus is on Sheet panel, Coil panel, Scrap panel, or any individual input field.
**Evidence:** Test each shortcut with focus on at least 3 different panels/fields.

### VAL-KEYS-008: Shortcuts do not interfere with normal text input
Typing regular characters in input fields does not trigger shortcuts. Ctrl+H while typing in a text field opens history (does not insert 'h').
**Pass condition:** Non-shortcut keystrokes are handled normally by input fields. Shortcut keystrokes are intercepted before reaching the text input.
**Evidence:** Type "48" in width field → only "48" appears. Press Ctrl+H in width field → history opens, no 'h' inserted.

---

## Area: Tooltips (UX)

### VAL-UX-012: Tooltips appear on hover after delay
Hovering over an input field for a brief delay (e.g., 500ms–1s) shows a tooltip with descriptive text near the cursor.
**Pass condition:** A tooltip popup appears after the hover delay with the field's help text.
**Evidence:** Hover over the Width input → after delay, tooltip "Enter width in inches (supports decimals)" appears.

### VAL-UX-013: Tooltips disappear on mouse leave
Moving the mouse away from an input field dismisses the tooltip immediately.
**Pass condition:** The tooltip popup is no longer visible after the mouse leaves the field.
**Evidence:** Hover over Width input → tooltip appears → move mouse to another field → tooltip disappears.

### VAL-UX-014: Each input field has an appropriate tooltip
All input fields display tooltips with contextually correct help text:
- Table combo: "Select the material type"
- Gauge combo: "Select gauge or size (supports fractions like 3/16)"
- Width: "Enter width in inches (supports decimals)"
- Length: "Enter length in inches (supports decimals)"
- Qty: "Enter quantity (integer >= 0)"
- PSF: "Enter lb/ft² directly (non-negative)"
- Thickness: "Enter thickness in inches (> 0)"
- Price mode: "Select pricing mode"
- Price value: "Enter unit price (>= 0)"
- Markup: "Enter markup percentage (e.g., 15 for 15%)"
- Tax: "Enter sales tax percentage (e.g., 8.75)"
- Setup fee: "Enter flat setup fee (>= 0)"
- Min order: "Enter minimum order amount (>= 0)"
- Coil width: "Enter coil width in inches"
- Coil thickness: "Enter coil thickness in inches (> 0)"
- Coil ID: "Enter coil inner diameter in inches (> 0)"
- Coil weight: "Enter coil weight in lb (> 0)"

**Pass condition:** Each listed field's tooltip matches the specified text exactly.
**Evidence:** Hover over each field → verify tooltip text matches the list above.

### VAL-UX-015: Tooltips do not interfere with input
While a tooltip is displayed, the user can still type in the field and the tooltip does not block the input area.
**Pass condition:** Input is accepted and correctly reflected in the field even while a tooltip is visible.
**Evidence:** Hover over width field → tooltip appears → type "48" → field shows "48".

### VAL-UX-016: Tooltip position is near the cursor
Tooltips appear at a position offset from the cursor (approximately +15px x, +15px y) so they don't obscure the field or the cursor itself.
**Pass condition:** The tooltip popup is visually positioned near but not overlapping the mouse cursor.
**Evidence:** Hover over several fields → observe tooltip position relative to cursor.
