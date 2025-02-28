const searchParams = new URLSearchParams(window.location.search);

const calamityClass = searchParams.get('class');
const stage = searchParams.get('stage');

if (calamityClass == null || stage == null) window.location = '/';

fetch(`/api/loadout/${calamityClass}/${stage}`)
  .then(response => new Promise((resolve, reject) => {
    if (!response.ok) return response.text().then(reject, reject);
    response.json().then(resolve, reject);
  }), handleError)
  .then(init, handleError);

function handleError(err) {
  console.error(err);

  const loadingText = document.getElementById('loading-text');
  loadingText.innerText = err;
  loadingText.classList.add('error');
}

/**
  * @param {{
      class: string,
      stage: string,
      stage_img: string,

      potion: string,
      powerups?: string[],
      armor: string,
      weapons: [string, string, string, string],
      equipment: string[],
      extra: { [key: string]: string[] },
  * }} data 
  */
function init(data) {
  document.getElementById('loading').style.display = 'none';
  document.getElementById('body').style.display = null;

  document.getElementById('title').innerText = `${data.class} - ${data.stage}`;
  document.getElementById('class').innerText = data.class;
  document.getElementById('stage').innerText = data.stage;
  document.getElementById('stage-img').src = data.stage_img;
  document.getElementById('class-img').src = `/assets/emoji/${calamityClass.toLowerCase()}.png`;

  document.getElementById('potion').innerText = data.potion;

  if (!data.powerups) {
    for (const powerups of document.getElementsByClassName('item powerups')) {
      powerups.style.display = 'none';
    }
    for (const potion of document.getElementsByClassName('item potion')) {
      potion.style['grid-column'] = 'span 2';
    }
  }

  appendListElements(document.getElementById('powerups'), data.powerups);
  document.getElementById('armor').innerText = data.armor;
  appendListElements(document.getElementById('weapons'), data.weapons);
  appendListElements(document.getElementById('equipment'), data.equipment);

  handleExtra(data.extra);
}

/**
  * @param {{ [key: string]: string[] }} extra 
  */
function handleExtra(extra) {
  const totalExtra = Object.keys(extra).length;
  let spacesLeft = 3;

  for (const [title, values] of Object.entries(extra)) {
    const loadout = document.getElementById('loadout');

    const item = document.createElement('div');
    item.classList.add('item');
    item.style.gridRow = `span ${Math.ceil(spacesLeft / totalExtra)}`;

    const label = document.createElement('div');
    label.classList.add('label');

    const h2 = document.createElement('h2');
    h2.innerText = title;

    label.appendChild(h2);
    item.appendChild(label);

    const value = document.createElement('div');
    value.classList.add('value');

    const ul = document.createElement('ul');
    appendListElements(ul, values);

    value.appendChild(ul);
    item.appendChild(value);
    loadout.appendChild(item);

    spacesLeft--;
  }
}

/**
  * @param {Element} ul 
  * @param {string[]?} elements 
  */
function appendListElements(ul, elements) {
  if (!elements) return;

  for (let element of elements) {
    // TODO: probably don't need regex here
    element = element.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    element = element.replace(/\*(.+?)\*/g, '<i>$1</i>');

    const li = document.createElement('li');

    const p = document.createElement('p');
    p.innerHTML = element;

    li.appendChild(p);

    ul.appendChild(li);
  }
}

