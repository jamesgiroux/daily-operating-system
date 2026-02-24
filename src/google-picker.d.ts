/**
 * Type declarations for the Google Picker API.
 * Loaded dynamically via gapi.load("picker").
 * Reference: https://developers.google.com/drive/picker/reference
 */

interface Window {
  gapi: {
    load: (
      api: string,
      options: { callback: () => void; onerror?: () => void }
    ) => void;
  };
  google: {
    picker: {
      DocsView: new () => google.picker.DocsView;
      PickerBuilder: new () => google.picker.PickerBuilder;
      Action: {
        PICKED: string;
        CANCEL: string;
      };
      Feature: {
        MULTISELECT_ENABLED: string;
      };
    };
  };
}

declare namespace google.picker {
  interface DocsView {
    setIncludeFolders(include: boolean): DocsView;
    setSelectFolderEnabled(enabled: boolean): DocsView;
  }

  interface PickerBuilder {
    addView(view: DocsView): PickerBuilder;
    setOAuthToken(token: string): PickerBuilder;
    enableFeature(feature: string): PickerBuilder;
    setCallback(callback: (data: ResponseObject) => void): PickerBuilder;
    build(): Picker;
  }

  interface Picker {
    setVisible(visible: boolean): void;
  }

  interface ResponseObject {
    action: string;
    docs: DocumentObject[];
  }

  interface DocumentObject {
    id: string;
    name: string;
    mimeType: string;
    url?: string;
  }
}
