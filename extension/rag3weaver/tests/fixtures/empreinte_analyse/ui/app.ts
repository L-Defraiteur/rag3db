import { Catalog, aide } from './index';

export function essai(): number {
    const c = Catalog.create();
    return aide() + (c ? 1 : 0);
}
