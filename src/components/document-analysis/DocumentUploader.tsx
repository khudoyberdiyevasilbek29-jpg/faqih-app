import { useCallback, useState } from "react";
import { useDropzone } from "react-dropzone";
import { FileUp } from "lucide-react";
import { Button } from "../shared/Button";
import {
  ACCEPTED_DOCUMENT_EXT,
  MAX_DOCUMENT_BYTES,
} from "../../lib/types";
import { pickDocumentFile } from "../../lib/api";

interface DocumentUploaderProps {
  disabled?: boolean;
  onPathSelected: (path: string, name: string) => void;
  error?: string | null;
}

function basename(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function isAllowedName(name: string): boolean {
  const lower = name.toLowerCase();
  return ACCEPTED_DOCUMENT_EXT.some((ext) => lower.endsWith(ext));
}

export function DocumentUploader({
  disabled,
  onPathSelected,
  error,
}: DocumentUploaderProps) {
  const [localError, setLocalError] = useState<string | null>(null);

  const validateAndEmit = useCallback(
    (path: string, size?: number) => {
      const name = basename(path);
      if (!isAllowedName(name)) {
        setLocalError("Faqat PDF, DOCX, TXT yoki MD fayllar qabul qilinadi.");
        return;
      }
      if (typeof size === "number" && size > MAX_DOCUMENT_BYTES) {
        setLocalError("Fayl hajmi 20 MB dan oshmasligi kerak.");
        return;
      }
      setLocalError(null);
      onPathSelected(path, name);
    },
    [onPathSelected],
  );

  const onDrop = useCallback(
    (accepted: File[]) => {
      const file = accepted[0];
      if (!file) return;
      const maybePath = (file as File & { path?: string }).path;
      if (maybePath) {
        validateAndEmit(maybePath, file.size);
        return;
      }
      setLocalError(
        "Brauzer fayl yo‘lini bermaydi. «Fayl tanlash» tugmasidan foydalaning.",
      );
    },
    [validateAndEmit],
  );

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    multiple: false,
    disabled,
    accept: {
      "application/pdf": [".pdf"],
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document": [
        ".docx",
      ],
      "text/plain": [".txt", ".md"],
    },
    maxSize: MAX_DOCUMENT_BYTES,
  });

  async function chooseFile() {
    setLocalError(null);
    const path = await pickDocumentFile();
    if (path) validateAndEmit(path);
  }

  const message = localError || error;

  return (
    <div className="space-y-4">
      <div
        {...getRootProps()}
        className={`glass glass-interactive relative cursor-pointer border-dashed p-6 ${
          isDragActive ? "brightness-105" : ""
        } ${disabled ? "pointer-events-none opacity-60" : ""}`}
      >
        <input {...getInputProps()} />
        <div className="relative z-[1] flex flex-col items-start gap-3 py-2">
          <h3 className="text-base font-semibold text-surface-ink">
            Hujjatni yuklang
          </h3>
          <p className="text-[15px] leading-relaxed text-surface-soft">
            PDF yoki DOCX ni shu yerga tashlang. Maksimum 20 MB. Tahlil faqat
            shu kompyuterda bajariladi.
          </p>
        </div>
      </div>
      <Button type="button" disabled={disabled} onClick={() => void chooseFile()}>
        <FileUp className="h-4 w-4" strokeWidth={1.5} />
        Fayl tanlash
      </Button>
      {message ? (
        <div className="glass relative px-4 py-3 text-sm text-rose-800 dark:text-rose-200">
          <p className="relative z-[1]">{message}</p>
        </div>
      ) : null}
    </div>
  );
}
