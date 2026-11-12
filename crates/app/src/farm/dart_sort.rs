//! Ordenação igual à do Dart (`List.sort`, sdk/lib/internal/sort.dart): insertion
//! sort até 33 elementos e dual-pivot quicksort acima. Não é estável: o mundo
//! sorteia na ordem dos empates (as árvores e as decorações), então a porta
//! replica o algoritmo do SDK em vez de usar um sort estável.
#![allow(dead_code)] // ponytail: usado pelo mundo; sem uso fora dele ainda
#![allow(clippy::indexing_slicing)] // porte 1:1 do SDK: todo índice fica entre `left` e `right`

use std::cmp::Ordering;

const INSERTION_SORT_THRESHOLD: i64 = 32;

pub fn sort_by<T: Copy>(a: &mut [T], cmp: &dyn Fn(&T, &T) -> Ordering) {
    if a.is_empty() {
        return;
    }
    do_sort(a, 0, a.len() as i64 - 1, cmp);
}

fn do_sort<T: Copy>(a: &mut [T], left: i64, right: i64, cmp: &dyn Fn(&T, &T) -> Ordering) {
    if right - left <= INSERTION_SORT_THRESHOLD {
        insertion_sort(a, left, right, cmp);
    } else {
        dual_pivot_quicksort(a, left, right, cmp);
    }
}

fn insertion_sort<T: Copy>(a: &mut [T], left: i64, right: i64, cmp: &dyn Fn(&T, &T) -> Ordering) {
    for i in left + 1..=right {
        let el = a[i as usize];
        let mut j = i;
        while j > left && cmp(&a[(j - 1) as usize], &el).is_gt() {
            a[j as usize] = a[(j - 1) as usize];
            j -= 1;
        }
        a[j as usize] = el;
    }
}

fn dual_pivot_quicksort<T: Copy>(
    a: &mut [T],
    left: i64,
    right: i64,
    cmp: &dyn Fn(&T, &T) -> Ordering,
) {
    let sixth = (right - left + 1) / 6;
    let index1 = left + sixth;
    let index5 = right - sixth;
    let index3 = (left + right) / 2;
    let index2 = index3 - sixth;
    let index4 = index3 + sixth;
    let mut el1 = a[index1 as usize];
    let mut el2 = a[index2 as usize];
    let mut el3 = a[index3 as usize];
    let mut el4 = a[index4 as usize];
    let mut el5 = a[index5 as usize];
    // rede de ordenação de 5 elementos
    let gt = |x: &T, y: &T| cmp(x, y).is_gt();
    if gt(&el1, &el2) {
        std::mem::swap(&mut el1, &mut el2);
    }
    if gt(&el4, &el5) {
        std::mem::swap(&mut el4, &mut el5);
    }
    if gt(&el1, &el3) {
        std::mem::swap(&mut el1, &mut el3);
    }
    if gt(&el2, &el3) {
        std::mem::swap(&mut el2, &mut el3);
    }
    if gt(&el1, &el4) {
        std::mem::swap(&mut el1, &mut el4);
    }
    if gt(&el3, &el4) {
        std::mem::swap(&mut el3, &mut el4);
    }
    if gt(&el2, &el5) {
        std::mem::swap(&mut el2, &mut el5);
    }
    if gt(&el2, &el3) {
        std::mem::swap(&mut el2, &mut el3);
    }
    if gt(&el4, &el5) {
        std::mem::swap(&mut el4, &mut el5);
    }
    let pivot1 = el2;
    let pivot2 = el4;
    a[index1 as usize] = el1;
    a[index3 as usize] = el3;
    a[index5 as usize] = el5;
    a[index2 as usize] = a[left as usize];
    a[index4 as usize] = a[right as usize];

    let mut less = left + 1;
    let mut great = right - 1;
    let pivots_are_equal = cmp(&pivot1, &pivot2).is_eq();
    if pivots_are_equal {
        let pivot = pivot1;
        // caso degenerado: bandeira holandesa com um pivô só
        let mut k = less;
        while k <= great {
            let ak = a[k as usize];
            let comp = cmp(&ak, &pivot);
            if comp.is_eq() {
                k += 1;
                continue;
            }
            if comp.is_lt() {
                if k != less {
                    a[k as usize] = a[less as usize];
                    a[less as usize] = ak;
                }
                less += 1;
            } else {
                loop {
                    let c = cmp(&a[great as usize], &pivot);
                    if c.is_gt() {
                        great -= 1;
                        continue;
                    } else if c.is_lt() {
                        a[k as usize] = a[less as usize];
                        a[less as usize] = a[great as usize];
                        less += 1;
                        a[great as usize] = ak;
                        great -= 1;
                        break;
                    } else {
                        a[k as usize] = a[great as usize];
                        a[great as usize] = ak;
                        great -= 1;
                        break;
                    }
                }
            }
            k += 1;
        }
    } else {
        let mut k = less;
        while k <= great {
            let ak = a[k as usize];
            if cmp(&ak, &pivot1).is_lt() {
                if k != less {
                    a[k as usize] = a[less as usize];
                    a[less as usize] = ak;
                }
                less += 1;
            } else if cmp(&ak, &pivot2).is_gt() {
                loop {
                    let c = cmp(&a[great as usize], &pivot2);
                    if c.is_gt() {
                        great -= 1;
                        if great < k {
                            break;
                        }
                        continue;
                    } else {
                        if cmp(&a[great as usize], &pivot1).is_lt() {
                            a[k as usize] = a[less as usize];
                            a[less as usize] = a[great as usize];
                            less += 1;
                            a[great as usize] = ak;
                            great -= 1;
                        } else {
                            a[k as usize] = a[great as usize];
                            a[great as usize] = ak;
                            great -= 1;
                        }
                        break;
                    }
                }
            }
            k += 1;
        }
    }
    a[left as usize] = a[(less - 1) as usize];
    a[(less - 1) as usize] = pivot1;
    a[right as usize] = a[(great + 1) as usize];
    a[(great + 1) as usize] = pivot2;
    do_sort(a, left, less - 2, cmp);
    do_sort(a, great + 2, right, cmp);
    if pivots_are_equal {
        return;
    }
    if less < index1 && great > index5 {
        while cmp(&a[less as usize], &pivot1).is_eq() {
            less += 1;
        }
        while cmp(&a[great as usize], &pivot2).is_eq() {
            great -= 1;
        }
        let mut k = less;
        while k <= great {
            let ak = a[k as usize];
            if cmp(&ak, &pivot1).is_eq() {
                if k != less {
                    a[k as usize] = a[less as usize];
                    a[less as usize] = ak;
                }
                less += 1;
            } else if cmp(&ak, &pivot2).is_eq() {
                loop {
                    let c = cmp(&a[great as usize], &pivot2);
                    if c.is_eq() {
                        great -= 1;
                        if great < k {
                            break;
                        }
                        continue;
                    } else {
                        if cmp(&a[great as usize], &pivot1).is_lt() {
                            a[k as usize] = a[less as usize];
                            a[less as usize] = a[great as usize];
                            less += 1;
                            a[great as usize] = ak;
                            great -= 1;
                        } else {
                            a[k as usize] = a[great as usize];
                            a[great as usize] = ak;
                            great -= 1;
                        }
                        break;
                    }
                }
            }
            k += 1;
        }
        do_sort(a, less, great, cmp);
    } else {
        do_sort(a, less, great, cmp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids na ordem final do `List.sort` do Dart (golden/sort_golden.dart):
    /// (n, valores i*37 % 11) e (n, valores (i*53 % 17) * 0.5), empates incluídos.
    fn ids(n: i64, f: impl Fn(i64) -> f64) -> Vec<i64> {
        let mut items: Vec<(f64, i64)> = (0..n).map(|i| (f(i), i)).collect();
        sort_by(&mut items, &|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal)
        });
        items.into_iter().map(|(_, id)| id).collect()
    }

    #[test]
    fn empates_na_mesma_ordem_do_dart() {
        assert_eq!(
            ids(20, |i| ((i * 37) % 11) as f64),
            [0, 11, 3, 14, 6, 17, 9, 1, 12, 4, 15, 7, 18, 10, 2, 13, 5, 16, 8, 19]
        );
        assert_eq!(
            ids(40, |i| ((i * 37) % 11) as f64),
            [
                22, 33, 11, 0, 25, 36, 3, 14, 17, 6, 39, 28, 9, 20, 31, 1, 12, 23, 34, 37, 15, 4,
                26, 7, 18, 29, 10, 21, 32, 2, 35, 24, 13, 27, 16, 5, 38, 19, 8, 30
            ]
        );
        assert_eq!(
            ids(60, |i| ((i * 37) % 11) as f64),
            [
                44, 55, 33, 11, 0, 22, 3, 14, 58, 25, 47, 36, 6, 39, 17, 28, 50, 9, 53, 20, 31, 42,
                1, 12, 34, 45, 56, 23, 15, 37, 26, 48, 4, 59, 29, 40, 18, 7, 51, 43, 10, 32, 21,
                54, 35, 24, 2, 13, 46, 57, 16, 38, 27, 5, 49, 19, 41, 30, 8, 52
            ]
        );
    }

    #[test]
    fn ordena_de_verdade() {
        let mut v: Vec<f64> = (0..200).map(|i| f64::from((i * 91) % 157)).collect();
        sort_by(&mut v, &|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        assert!(v.windows(2).all(|w| w[0] <= w[1]));
    }
}
