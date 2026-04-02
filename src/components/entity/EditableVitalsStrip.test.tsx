/** @vitest-environment jsdom */

import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { EditableVitalsStrip } from "./EditableVitalsStrip";
import type { PresetVitalField } from "@/types/preset";

// ── Mocks ──────────────────────────────────────────────────────────────────────

vi.mock("@/components/ui/date-picker", () => ({
  DatePicker: ({ value, onChange, placeholder }: { value?: string; onChange: (v: string) => void; placeholder?: string }) => (
    <input
      data-testid="date-picker"
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
    />
  ),
}));

// ── Test Data ──────────────────────────────────────────────────────────────────

const currencyField: PresetVitalField = {
  key: "arr",
  label: "ARR",
  fieldType: "currency",
  source: "column",
  columnMapping: "arr",
};

const selectField: PresetVitalField = {
  key: "health",
  label: "Health",
  fieldType: "select",
  source: "column",
  options: ["green", "yellow", "red"],
};

const dateField: PresetVitalField = {
  key: "contract_end",
  label: "Renewal Date",
  fieldType: "date",
  source: "column",
  columnMapping: "contract_end",
};

const textField: PresetVitalField = {
  key: "lifecycle",
  label: "Lifecycle",
  fieldType: "text",
  source: "column",
};

const signalField: PresetVitalField = {
  key: "meetingFrequency30d",
  label: "Meetings/30d",
  fieldType: "number",
  source: "signal",
};

const allFields = [currencyField, selectField, dateField, textField, signalField];

const entityData = {
  arr: 125000,
  health: "green",
  lifecycle: "onboarding",
  renewalDate: "2027-06-15",
  signals: { meetingFrequency30d: 4 },
};

// ── Tests ──────────────────────────────────────────────────────────────────────

describe("EditableVitalsStrip", () => {
  let onFieldChange: (key: string, columnMapping: string | undefined, source: string, value: string) => void;

  beforeEach(() => {
    onFieldChange = vi.fn();
  });

  it("returns null when fields array is empty", () => {
    const { container } = render(
      <EditableVitalsStrip fields={[]} entityData={{}} onFieldChange={onFieldChange} />,
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders with realistic entity data", () => {
    render(
      <EditableVitalsStrip fields={allFields} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    // Currency field shows formatted ARR
    expect(screen.getByText(/\$125K ARR/)).toBeInTheDocument();
    // Select field shows capitalized health
    expect(screen.getByText(/Green Health/)).toBeInTheDocument();
    // Signal field displays read-only
    expect(screen.getByText(/Meetings\/30d 4/)).toBeInTheDocument();
  });

  it("shows empty placeholders when entity data is missing", () => {
    render(
      <EditableVitalsStrip fields={[currencyField, selectField]} entityData={{}} onFieldChange={onFieldChange} />,
    );

    // Empty fields show their label as placeholder
    expect(screen.getByText("ARR")).toBeInTheDocument();
    expect(screen.getByText("Health")).toBeInTheDocument();
  });

  it("opens inline input on click for currency field", () => {
    render(
      <EditableVitalsStrip fields={[currencyField]} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    const value = screen.getByText(/\$125K ARR/);
    fireEvent.click(value);

    // Input should appear with the raw value
    const input = screen.getByDisplayValue("125000");
    expect(input).toBeInTheDocument();
  });

  it("commits value on Enter in inline input", () => {
    render(
      <EditableVitalsStrip fields={[currencyField]} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    fireEvent.click(screen.getByText(/\$125K ARR/));
    const input = screen.getByDisplayValue("125000");
    fireEvent.change(input, { target: { value: "200000" } });
    fireEvent.keyDown(input, { key: "Enter" });

    expect(onFieldChange).toHaveBeenCalledWith("arr", "arr", "column", "200000");
  });

  it("opens select dropdown on click for select field", () => {
    render(
      <EditableVitalsStrip fields={[selectField]} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    fireEvent.click(screen.getByText(/Green Health/));

    // Select element should appear with options
    const select = screen.getByRole("combobox");
    expect(select).toBeInTheDocument();

    // Change selection
    fireEvent.change(select, { target: { value: "red" } });
    expect(onFieldChange).toHaveBeenCalledWith("health", undefined, "column", "red");
  });

  it("renders extra vitals as read-only items", () => {
    render(
      <EditableVitalsStrip
        fields={[currencyField]}
        entityData={entityData}
        onFieldChange={onFieldChange}
        extraVitals={[{ text: "3 meetings / 30d", highlight: "turmeric" }]}
      />,
    );

    expect(screen.getByText("3 meetings / 30d")).toBeInTheDocument();
  });

  it("hides empty signal fields", () => {
    render(
      <EditableVitalsStrip
        fields={[signalField]}
        entityData={{ signals: {} }}
        onFieldChange={onFieldChange}
      />,
    );

    // Signal field with no value should not render
    expect(screen.queryByText("Meetings/30d")).not.toBeInTheDocument();
  });

  it("shows date field and opens picker on click", () => {
    render(
      <EditableVitalsStrip fields={[dateField]} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    // Date should show as renewal countdown
    const dateDisplay = screen.getByText(/Renewal in/);
    expect(dateDisplay).toBeInTheDocument();

    fireEvent.click(dateDisplay);
    // Date picker should appear
    expect(screen.getByTestId("date-picker")).toBeInTheDocument();
  });

  it("cancels editing on Escape", () => {
    render(
      <EditableVitalsStrip fields={[currencyField]} entityData={entityData} onFieldChange={onFieldChange} />,
    );

    fireEvent.click(screen.getByText(/\$125K ARR/));
    const input = screen.getByDisplayValue("125000");
    fireEvent.keyDown(input, { key: "Escape" });

    // Should return to display mode
    expect(screen.queryByDisplayValue("125000")).not.toBeInTheDocument();
    expect(screen.getByText(/\$125K ARR/)).toBeInTheDocument();
  });
});
