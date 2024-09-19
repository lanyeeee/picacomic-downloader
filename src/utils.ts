import {commands} from "./bindings.ts";

export async function showPathInFileManager(path: string | undefined) {
    if (path === undefined) {
        return;
    }
    await commands.showPathInFileManager(path);
}