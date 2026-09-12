import { useState, type FormEvent, type KeyboardEvent } from "react";
import { Button } from "../shared/Button";
import { LANGUAGE_LABELS, type LanguagePreference } from "../../lib/types";

interface InputBarProps {
  onSend: (value: string) => void;
  disabled?: boolean;
  language: LanguagePreference;
  placeholder?: string;
}

export function InputBar({
  onSend,
  disabled = false,
  language,
  placeholder = "Savolingizni yozing…",
}: InputBarProps) {
  const [value, setValue] = useState("");

  function submit() {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setValue("");
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    submit();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  }

  return (
    <form onSubmit={handleSubmit} className="glass relative p-4">
      <div className="relative z-[1] mb-3 flex items-center justify-between gap-2 px-1">
        <span className="text-[11px] text-surface-faint">
          Til: {LANGUAGE_LABELS[language]}
        </span>
        <span className="text-[11px] text-surface-faint">Enter — yuborish</span>
      </div>
      <div className="relative z-[1] flex items-end gap-3">
        <textarea
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={handleKeyDown}
          rows={2}
          disabled={disabled}
          placeholder={placeholder}
          className="min-h-[56px] flex-1 resize-none bg-transparent text-[15px] leading-relaxed text-surface-ink outline-none placeholder:text-surface-faint"
        />
        <Button type="submit" disabled={disabled || !value.trim()}>
          Yuborish
        </Button>
      </div>
    </form>
  );
}
