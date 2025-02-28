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
}

/**
  * @param {Element} ol 
  * @param {string[]?} elements 
  */
function appendListElements(ol, elements) {
  if (!elements) return;

  for (let element of elements) {
    element = element.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    element = element.replace(/\*(.+?)\*/g, '<i>$1</i>');

    const li = document.createElement('li');
    // TODO: probably don't need regex here
    li.innerHTML = element;

    ol.appendChild(li);
  }
}

